#[macro_use]
extern crate lazy_static;

/// Get information about a process (/proc/[pid]/)
pub mod pid;
/// The error type used for this crate
pub mod error;
/// Get informmation about system memory
pub mod meminfo;

/// The type used to repesent pids
pub type TaskId = i32;
/// The type used to repesent memory (in bytes)
pub type MemSize = u64;
