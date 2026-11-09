//! Application composition root.
//! Dependencies point inward: adapters -> application/runtime -> domain.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod runtime;

pub use adapters::tauri::bootstrap::run;
