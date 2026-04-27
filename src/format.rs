use humansize::{format_size, BINARY};

pub fn file_size(bytes: u64) -> String {
    format_size(bytes, BINARY)
}

pub fn token_count(n: usize) -> String {
    n.to_string()
}
