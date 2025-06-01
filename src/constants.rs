use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref EMAIL_REGEX: Regex = Regex::new(r"(?i)[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}").unwrap();
} 