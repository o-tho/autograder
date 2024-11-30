#![cfg(target_arch = "wasm32")]
pub mod create_form;
pub mod create_key;
pub mod create_magic_link;
pub mod create_template;
pub mod generate_report;
pub mod help;
pub mod utils;
pub mod webapp;

pub use webapp::WebApp;
