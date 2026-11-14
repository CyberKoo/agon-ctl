#![cfg(target_os = "linux")]

mod display;
mod i2c;

pub use agon_osd::DisplayInfo;
pub use display::{LinuxDisplay, LinuxDisplayError, enumerate, enumerate_with_diagnostics};
