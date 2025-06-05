use regex::Regex;
use lazy_static::lazy_static;
use std::fs::{File, OpenOptions};
use std::sync::Mutex;

pub const BUFFER_SIZE_OPTIMIZED: usize = 512 * 1024;
pub const BUFFER_SIZE_ULTRA: usize = 1024 * 1024;
pub const CHANNEL_BUFFER: usize = 10_000;
pub const HASHMAP_INITIAL_CAPACITY_OPTIMIZED: usize = 1_000_000;

pub const BYTES_TO_KB: u64 = 1024;
pub const BYTES_TO_GB: f64 = 1_073_741_824.0;
pub const KB_TO_GB: f64 = 1_048_576.0;
pub const PERCENT_DIVISOR: u64 = 100;
pub const EMAIL_PARTS_COUNT: usize = 2;

pub const LOCAL_USERS_CAPACITY: usize = 10_000;
pub const BATCH_SIZE_OPTIMIZED: usize = 10;
pub const CHUNK_MULTIPLIER: usize = 4;
pub const WARNING_THRESHOLD_PERCENT: usize = 8;
pub const WARNING_THRESHOLD_DIVISOR: usize = 10;
pub const WARNING_CHECK_INTERVAL: usize = 10_000;

pub const EMERGENCY_MEMORY_LIMIT_GB: f64 = 8.0;
pub const MAX_RECORDS_SAFETY_LIMIT: usize = 250_000;

lazy_static! {
    pub static ref EMAIL_REGEX: Regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
    
    pub static ref LOG_FILE: Mutex<File> = Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open("processing_errors.log")
            .expect("Failed to open log file")
    );
    
    pub static ref VERBOSE_MODE: Mutex<bool> = Mutex::new(false);
} 