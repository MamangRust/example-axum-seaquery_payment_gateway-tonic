# 💳 Example Payment Gateway (Rust)

Proyek ini adalah contoh implementasi **sistem payment gateway sederhana** yang dibangun dengan bahasa pemrograman **Rust**. Tujuan utama proyek ini adalah memberikan gambaran bagaimana merancang layanan finansial dengan pendekatan **service-oriented**, komunikasi efisien via **gRPC**, serta mendukung **observability** sejak awal.

---

## ⚡ Fitur Utama

### 🔐 1. Manajemen Pengguna

* Registrasi & login pengguna.
* Penyimpanan data pengguna di PostgreSQL.
* Tokenisasi session bisa dikombinasikan dengan Redis untuk performa.

### 💰 2. Manajemen Saldo

* Setiap pengguna memiliki akun saldo di database.
* Mendukung operasi:

  * **Top-up** (menambah saldo).
  * **Withdraw** (menarik saldo).
  * **Transfer** (antar pengguna, debit & kredit dalam 1 transaksi).

### 📑 3. Transaksi

* Semua aktivitas finansial dicatat di **riwayat transaksi**.
* Dukungan query transaksi per pengguna.
* Konsistensi dijaga dengan transaksi database (SQLx).

### ⚡ 4. Kinerja & Komunikasi

* **Client → Server** pakai gRPC (hemat bandwidth & cepat).
* Query SQL dibangun dinamis dengan **SeaQuery**.
* **sqlx** sebagai runtime query, mendukung migrasi & pooling connection.

### 📊 5. Observability (Monitoring & Logging)

* Metrik request, error, latency dikirim ke **Prometheus**.
* Log request/response dikirim ke **Loki**.
* Semua data divisualisasikan lewat **Grafana Dashboard**.

---

## 🎯 Keunggulan Proyek

* **Ringan & Cepat** → Rust + gRPC untuk high-performance backend.
* **Terukur** → bisa berkembang dari sederhana ke kompleks (misalnya menambahkan layanan fraud detection atau notifikasi).
* **Modular** → pemisahan jelas antara service publik (Client) dan core bisnis (Server).
* **Observability-ready** → bisa langsung dipantau performa & log-nya.
* **Cocok untuk Belajar** → mengajarkan konsep dasar **payment system**, **event-driven design**, dan **observability** dalam Rust.

## Arsitektur Sistem

Proyek ini dibagi menjadi dua layanan utama:

1. **Client Service (Axum REST API)**

   * Berfungsi sebagai **gerbang publik** untuk menerima request dari pengguna.
   * Menyediakan endpoint REST (HTTPS) agar mudah diakses oleh aplikasi web/mobile.
   * Melakukan validasi input sebelum meneruskan request ke layanan internal.

2. **Server Service (Tonic gRPC)**

   * Menangani seluruh **logika bisnis inti**: registrasi, top-up, transfer, withdraw, pengecekan saldo.
   * Komunikasi antar service menggunakan **gRPC** untuk kecepatan dan efisiensi.
   * Berinteraksi dengan **PostgreSQL** sebagai database persisten dan **Redis** sebagai cache.

3. **Observability Stack**

   * **Prometheus** untuk metrics (latency, error rate, request count).
   * **Loki** untuk logging terpusat.
   * **Grafana** sebagai dashboard untuk visualisasi metrik & log.


```mermaid
graph TD
    subgraph "Infrastruktur"
        DB[(PostgreSQL)]
        Cache[(Redis)]
    end

    subgraph "Layanan Aplikasi"
        Client[Client Service / Public API <br> Axum]
        Server[Server Service / Core Logic <br> Tonic gRPC]
    end

    subgraph "Observability"
        Prometheus[Prometheus]
        Grafana[Grafana]
        Loki[Loki]
    end

    User[Pengguna] -->|HTTPS Request| Client
    Client -->|gRPC Call| Servero
    Server -->|SQL Query| DB
    Server -->|Cache Ops| Cache
    
    Client -->|Metrics| Prometheus
    Server -->|Metrics| Prometheus
    Client -->|Logs| Loki
    Server -->|Logs| Loki
    Grafana -->|Query| Prometheus
    Grafana -->|Query| Loki

```

-   **Pengguna**: Mengirim permintaan (misalnya, registrasi, transfer, top-up) melalui REST API ke `Client Service`.
-   **Client Service (Axum)**: Menerima permintaan HTTP, melakukan validasi awal, dan meneruskannya ke `Server Service` melalui panggilan gRPC.
-   **Server Service (Tonic)**: Menerima panggilan gRPC, memproses seluruh logika bisnis, berinteraksi dengan database (PostgreSQL) dan cache (Redis), lalu mengembalikan hasilnya.
-   **Database (PostgreSQL)**: Menyimpan data persisten seperti informasi pengguna, saldo, dan riwayat transaksi.
-   **Observability**: Metrik dari kedua layanan dikumpulkan oleh Prometheus, dan log dikumpulkan oleh Loki. Keduanya dapat divisualisasikan menggunakan Grafana.

## Entity-Relationship Diagram (ERD)

Diagram berikut menggambarkan hubungan antar tabel dalam database.

```mermaid
erDiagram
    users {
        INT user_id PK
        VARCHAR firstname
        VARCHAR lastname
        VARCHAR email UK
        VARCHAR password
        VARCHAR noc_transfer UK
    }

    saldo {
        INT saldo_id PK
        INT user_id FK
        INT total_balance
        INT withdraw_amount
        TIMESTAMP withdraw_time
    }

    topups {
        INT topup_id PK
        INT user_id FK
        TEXT topup_no
        INT topup_amount
        TEXT topup_method
        TIMESTAMP topup_time
    }

    transfers {
        INT transfer_id PK
        INT transfer_from FK
        INT transfer_to FK
        INT transfer_amount
        TIMESTAMP transfer_time
    }

    withdraws {
        INT withdraw_id PK
        INT user_id FK
        INT withdraw_amount
        TIMESTAMP withdraw_time
    }

    users ||--o{ saldo : "memiliki"
    users ||--o{ topups : "melakukan"
    users ||--o{ withdraws : "melakukan"
    users ||--o{ transfers : "mengirim"
    users ||--o{ transfers : "menerima"
```

## Cara Penggunaan

Proyek ini dapat dijalankan secara lokal maupun menggunakan Docker.

### Menjalankan dengan Docker (Rekomendasi)

Ini adalah cara termudah untuk menjalankan keseluruhan sistem, termasuk database dan layanan observability.

1.  **Prasyarat**:
    *   [Docker](https://www.docker.com/get-started)
    *   [Docker Compose](https://docs.docker.com/compose/install/)

2.  **Langkah-langkah**:
    *   Clone repositori ini.
    *   Salin file `.env.example` menjadi `.env` jika ada, atau pastikan konfigurasi di `docker-compose.yaml` sudah sesuai.
    *   Buka terminal di root direktori proyek dan jalankan:
        ```bash
        docker-compose up --build -d
        ```
    *   Perintah ini akan membangun *image* untuk `client` dan `server`, lalu menjalankan semua layanan (termasuk PostgreSQL, Redis, Prometheus, dll.) di latar belakang.
    *   Untuk menghentikan semua layanan, jalankan:
        ```bash
        docker-compose down
        ```

### Menjalankan di Lingkungan Lokal

Untuk menjalankan layanan secara manual di mesin lokal Anda.

1.  **Prasyarat**:
    *   Instalasi [Rust](https://www.rust-lang.org/tools/install).
    *   Instalasi [PostgreSQL](https://www.postgresql.org/download/) dan [Redis](https://redis.io/docs/getting-started/installation/).
    *   Install `sqlx-cli` untuk migrasi database:
        ```bash
        cargo install sqlx-cli --no-default-features --features rustls,postgres
        ```

2.  **Langkah-langkah**:
    *   Clone repositori ini.
    *   Buat database baru di PostgreSQL untuk proyek ini.
    *   Salin file `.env.example` menjadi `.env` dan isi variabel lingkungan, terutama `DATABASE_URL` dan `REDIS_URL`. Contoh:
        ```
        DATABASE_URL="postgresql://user:password@localhost:5432/payment_db"
        REDIS_URL="redis://127.0.0.1/"
        ```
    *   Jalankan migrasi database untuk membuat tabel-tabel yang diperlukan:
        ```bash
        sqlx migrate run
        ```
    *   Buka terminal baru, jalankan **Server Service**:
        ```bash
        cargo run --bin server
        ```
    *   Buka terminal lainnya, jalankan **Client Service**:
        ```bash
        cargo run --bin client
        ```
    *   Sekarang, `Client Service` akan berjalan di `http://127.0.0.1:8000` (atau sesuai konfigurasi) dan siap menerima permintaan.

## Screnshoot

### OpenApi

<img src="./images/openapi.png" />

### Jaeger

<img src="./images/Jaeger.png" />


### Prometheus

<img src="./images/prometheus.png" />


### Metrics

#### Metrics Server

<img src="./images/metrics_server.png" />

#### Metrics Client

<img src="./images/metrics_client.png" />


## Loki

<img src="./images/loki.png" />

## Grafana Prometheus

### Server Metrics

<img src="./images/grafana-server.png"/>

### Client Metrics

<img src="./images/grafana-client.png" />

## Node-Exporter

<img src="./images/node-expoter.png" />

## Otel-Collector

<img src="./images/otel.png" />
