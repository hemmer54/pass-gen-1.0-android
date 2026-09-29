#[cfg(target_os = "android")]
mod android;
mod app;
mod password_core;

pub use app::PasswordApp;
