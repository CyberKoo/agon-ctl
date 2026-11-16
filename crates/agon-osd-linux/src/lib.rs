#![cfg(target_os = "linux")]

mod discovery;
mod display;
mod edid;
mod error;
mod i2c;
mod naming;
mod probe;
#[cfg(test)]
mod test_support;

pub use agon_osd::DisplayInfo;
pub use discovery::{enumerate, enumerate_with_diagnostics};
pub use display::LinuxDisplay;
pub use error::LinuxDisplayError;
