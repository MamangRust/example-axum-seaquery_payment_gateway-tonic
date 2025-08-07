use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::metrics::{counter::Counter, family::Family, gauge::Gauge};
use prometheus_client::registry::Registry;
use prometheus_client_derive_encode::{EncodeLabelSet, EncodeLabelValue};
use std::{
    fs,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::System;

fn get_thread_count(pid: usize) -> Option<i64> {
    let path = format!("/proc/{pid}/status");
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            if line.starts_with("Threads:") && line.split_whitespace().nth(1).is_some() {
                return line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|thread_count| thread_count.parse::<i64>().ok());
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub memory_alloc_bytes: Gauge,
    pub memory_sys_bytes: Gauge,
    pub available_memory: Counter,
    pub thread_usage: Gauge,
    pub total_cpu_usage: Counter,
    pub process_start_time: Gauge,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMetrics {
    pub fn new() -> Self {
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        let metrics = Self {
            memory_alloc_bytes: Gauge::default(),
            memory_sys_bytes: Gauge::default(),
            available_memory: Counter::default(),
            thread_usage: Gauge::default(),
            total_cpu_usage: Counter::default(),
            process_start_time: Gauge::default(),
        };

        metrics.process_start_time.set(start_time as i64);
        metrics
    }

    pub fn register(&self, registry: &mut Registry) {
        registry.register(
            "process_memory_alloc_bytes",
            "Current memory allocation in bytes",
            self.memory_alloc_bytes.clone(),
        );

        registry.register(
            "process_memory_sys_bytes",
            "Total system memory in bytes",
            self.memory_sys_bytes.clone(),
        );

        registry.register(
            "process_memory_frees_total",
            "Total Available Memory",
            self.available_memory.clone(),
        );

        registry.register(
            "process_thread_total",
            "Thread total",
            self.thread_usage.clone(),
        );

        registry.register(
            "total_cpu_usage",
            "Total cpu usage",
            self.total_cpu_usage.clone(),
        );

        registry.register(
            "process_start_time_seconds",
            "Start time of the process since unix epoch in seconds",
            self.process_start_time.clone(),
        );
    }

    pub async fn update_metrics(&self) {
        let mut sys = System::new_all();
        sys.refresh_all();

        let pid = std::process::id() as usize;

        if let Some(process) = sys.process(sysinfo::Pid::from(pid)) {
            let current_memory = process.memory() as i64;
            self.memory_alloc_bytes.set(current_memory);
            self.memory_sys_bytes.set(process.virtual_memory() as i64);

            let available_memory = sys.available_memory() / 1_024;
            self.available_memory.inc_by(available_memory);

            let total_cpu_usage = sys.global_cpu_usage();
            self.total_cpu_usage.inc_by(total_cpu_usage as u64);

            if let Some(thread_count) = get_thread_count(pid) {
                self.thread_usage.set(thread_count);
            }
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum Status {
    Success,
    Error,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct Labels {
    pub method: Method,
    pub status: Status,
}

#[derive(Clone, Debug)]
pub struct Metrics {
    pub request_counter: Family<Labels, Counter>,
    pub request_duration: Family<Labels, Histogram>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            request_counter: Family::default(),
            request_duration: Family::new_with_constructor(|| {
                Histogram::new(vec![
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ])
            }),
        }
    }

    pub fn record(&self, method: Method, status: Status, duration_secs: f64) {
        let labels = Labels { method, status };
        self.request_counter.get_or_create(&labels).inc();
        self.request_duration
            .get_or_create(&labels)
            .observe(duration_secs);
    }
}

pub async fn run_metrics_collector(system_metrics: Arc<SystemMetrics>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
    loop {
        interval.tick().await;
        system_metrics.update_metrics().await;
    }
}
