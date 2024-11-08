#![cfg(target_arch = "wasm32")]
pub mod create_key;
pub mod create_template;
pub mod generate_report;
pub mod help;
pub mod utils;
pub mod webapp;

pub use webapp::WebApp;
