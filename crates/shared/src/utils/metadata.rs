use opentelemetry::propagation::Injector;
use tonic::metadata::{MetadataKey, MetadataMap, MetadataValue};

pub struct MetadataInjector<'a>(pub &'a mut MetadataMap);

impl<'a> Injector for MetadataInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(metadata_key) = key.parse::<MetadataKey<_>>()
            && let Ok(metadata_value) = value.parse::<MetadataValue<_>>()
        {
            self.0.insert(metadata_key, metadata_value);
        }
    }
}
