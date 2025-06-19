// Copyright (c) 2025, TheByteSlayer, Hydrogen
// A scalable and lightweight Key Value Cache written in Rust

use tracing::info;

pub fn log_set_endpoint(key: &str, value: &str) {
    info!("SET {} {}", key, value);
}

pub fn log_get_endpoint(key: &str) {
    info!("GET {}", key);
}

pub fn log_delete_endpoint(key: &str) {
    info!("DEL {}", key);
}

pub fn log_invalid_endpoint(command: &str) {
    info!("Invalid endpoint: {}", command);
}

pub fn log_invalid_utf8() {
    info!("Invalid UTF-8 request");
} 