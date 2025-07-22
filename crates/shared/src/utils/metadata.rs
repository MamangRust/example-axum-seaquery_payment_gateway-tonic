use opentelemetry::propagation::Injector;
use tonic::metadata::MetadataMap;

pub struct MetadataInjector<'a>(pub &'a mut MetadataMap);

impl<'a> Injector for MetadataInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(metadata_key) = key.parse::<tonic::metadata::MetadataKey<_>>() {
            if let Ok(metadata_value) = value.parse::<tonic::metadata::MetadataValue<_>>() {
                self.0.insert(metadata_key, metadata_value);
            }
        }
    }
}
