#![cfg(target_os = "macos")]

mod display;
mod framework;

pub use agon_osd::DisplayInfo;
pub use display::{MacDisplay, MacDisplayError, enumerate};
