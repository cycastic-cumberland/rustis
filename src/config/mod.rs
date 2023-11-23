use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApplicationConfig {
    pub log_level: String,
    pub partition_count: u16
}