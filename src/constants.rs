//! Crate-wide constants for log limits and I/O caps.

pub const MAX_LINES: usize = 150;

/// Max bytes per line when reading; avoids OOM on files with one huge line.
pub const MAX_LINE_LEN: usize = 64 * 1024; // 64 KiB

/// Max bytes to read per poll in live mode.
pub const POLL_READ_CAP: usize = 512 * 1024; // 512 KiB

/// When file is larger than this, we only read the last TAIL_READ_SIZE bytes (no full-file stream).
pub const TAIL_READ_SIZE: u64 = 2 * 1024 * 1024; // 2 MiB
