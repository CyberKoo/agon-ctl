#![cfg(target_os = "linux")]

use libc::{c_int, c_ulong};

pub const I2C_SLAVE: c_ulong = 0x0703;
pub const I2C_SLAVE_FORCE: c_ulong = 0x0706;

unsafe extern "C" {
    pub fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}
