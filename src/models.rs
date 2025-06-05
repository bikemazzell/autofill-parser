use serde::{Serialize, Deserialize};
use std::collections::HashMap;

pub type RawRecord = HashMap<String, String>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UserOutput {
    pub identifier: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<String>,
    #[serde(flatten)]
    pub other_fields: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub memory_usage_percent: usize,
    pub temp_directory: String,
    pub progress_update_frequency: usize,
    pub max_records_before_swap: usize,
    pub memory_check_interval_secs: u64,
    pub record_check_interval: usize,
    pub hashmap_initial_capacity: usize,
    pub safety_records_limit: usize,
    pub memory_pressure_threshold_gb: f64,
    pub chunk_size_multiplier: usize,
    pub small_dataset_threshold_gb: f64,
    pub large_dataset_threshold_gb: f64,
    pub emergency_abort_threshold_gb: f64,
    pub max_file_size_bytes: u64,
    pub single_threaded_threshold_gb: f64,
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.memory_usage_percent == 0 || self.memory_usage_percent > 95 {
            return Err(format!("memory_usage_percent must be between 1 and 95, got {}", self.memory_usage_percent));
        }


        if self.max_records_before_swap == 0 {
            return Err("max_records_before_swap must be greater than 0".to_string());
        }
        if self.safety_records_limit == 0 {
            return Err("safety_records_limit must be greater than 0".to_string());
        }
        if self.safety_records_limit > self.max_records_before_swap {
            return Err(format!("safety_records_limit ({}) should be <= max_records_before_swap ({})", 
                self.safety_records_limit, self.max_records_before_swap));
        }


        if self.memory_pressure_threshold_gb <= 0.0 {
            return Err("memory_pressure_threshold_gb must be positive".to_string());
        }
        if self.emergency_abort_threshold_gb <= 0.0 {
            return Err("emergency_abort_threshold_gb must be positive".to_string());
        }
        if self.emergency_abort_threshold_gb >= self.memory_pressure_threshold_gb {
            return Err(format!("emergency_abort_threshold_gb ({:.2}) must be < memory_pressure_threshold_gb ({:.2})", 
                self.emergency_abort_threshold_gb, self.memory_pressure_threshold_gb));
        }


        if self.small_dataset_threshold_gb <= 0.0 {
            return Err("small_dataset_threshold_gb must be positive".to_string());
        }
        if self.large_dataset_threshold_gb <= self.small_dataset_threshold_gb {
            return Err(format!("large_dataset_threshold_gb ({:.2}) must be > small_dataset_threshold_gb ({:.2})", 
                self.large_dataset_threshold_gb, self.small_dataset_threshold_gb));
        }


        if self.memory_check_interval_secs == 0 {
            return Err("memory_check_interval_secs must be greater than 0".to_string());
        }
        if self.record_check_interval == 0 {
            return Err("record_check_interval must be greater than 0".to_string());
        }
        if self.progress_update_frequency == 0 {
            return Err("progress_update_frequency must be greater than 0".to_string());
        }


        if self.hashmap_initial_capacity == 0 {
            return Err("hashmap_initial_capacity must be greater than 0".to_string());
        }
        if self.chunk_size_multiplier == 0 {
            return Err("chunk_size_multiplier must be greater than 0".to_string());
        }


        if self.max_file_size_bytes == 0 {
            return Err("max_file_size_bytes must be greater than 0".to_string());
        }


        if self.single_threaded_threshold_gb < 0.0 {
            return Err("single_threaded_threshold_gb must be non-negative".to_string());
        }


        if self.temp_directory.is_empty() {
            return Err("temp_directory cannot be empty".to_string());
        }

        Ok(())
    }

    pub fn with_defaults() -> Self {
        Self {
            memory_usage_percent: 50,
            temp_directory: "temp".to_string(),
            progress_update_frequency: 10000,
            max_records_before_swap: 500000,
            memory_check_interval_secs: 5,
            record_check_interval: 10000,
            hashmap_initial_capacity: 500000,
            safety_records_limit: 250000,
            memory_pressure_threshold_gb: 2.0,
            chunk_size_multiplier: 2,
            small_dataset_threshold_gb: 1.0,
            large_dataset_threshold_gb: 10.0,
            emergency_abort_threshold_gb: 1.0,
            max_file_size_bytes: 10_737_418_240,
            single_threaded_threshold_gb: 0.5,
        }
    }
}