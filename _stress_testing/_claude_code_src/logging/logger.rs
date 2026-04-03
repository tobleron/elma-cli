// Shared logging wrapper utility
use std::fmt;
use std::io::{self, Write};

pub fn log_info(msg: impl fmt::Display) {
    println!("[INFO] {}", msg);
}

pub fn log_error(msg: impl fmt::Display) {
    eprintln!("[ERROR] {}", msg);
}

pub fn log_debug(msg: impl fmt::Display) {
    if std::env::var("RUST_LOG").unwrap_or_default().contains("debug") {
        println!("[DEBUG] {}", msg);
    }
}

pub fn log_trace(msg: impl fmt::Display) {
    if std::env::var("RUST_LOG").unwrap_or_default().contains("trace") {
        println!("[TRACE] {}", msg);
    }
}
