use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApplicationConfig {
    pub log_level: String,
    pub partition_count: u16,
    pub regex_partition_count: u16,
    pub regex_partition_capacity: u16,
}