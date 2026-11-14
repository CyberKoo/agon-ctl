use agon_osd_linux_sys::{I2C_SLAVE, ioctl};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

pub(crate) struct I2cDevice {
    file: File,
}

impl I2cDevice {
    pub(crate) fn open(path: &Path, slave_address: u16) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        set_slave_address(&file, slave_address)?;
        Ok(Self { file })
    }

    pub(crate) fn write_packet(&mut self, packet: &[u8]) -> io::Result<()> {
        self.file.write_all(packet)
    }

    pub(crate) fn read_packet(&mut self, response: &mut [u8]) -> io::Result<()> {
        self.file.read_exact(response)
    }
}

fn set_slave_address(file: &File, address: u16) -> io::Result<()> {
    // SAFETY: the fd is valid, I2C_SLAVE expects an integer address, and no pointer is borrowed.
    let result = unsafe { ioctl(file.as_raw_fd(), I2C_SLAVE, i32::from(address)) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
