use agon_osd::{PacketTransport, TransportError};
use agon_osd_linux_sys::{I2C_SLAVE, I2C_SLAVE_FORCE, ioctl};
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

const DDC_CI_DATA_ADDRESS: u8 = 0x51;

pub(crate) struct I2cDevice {
    file: File,
}

impl I2cDevice {
    pub(crate) fn open(path: &Path, slave_address: u16) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        set_slave_address(&file, slave_address)?;
        Ok(Self { file })
    }

    // NVIDIA's proprietary driver rejects i2c-dev I2C_RDWR; keep using file I/O.
    pub(crate) fn write_all(&mut self, packet: &[u8]) -> io::Result<()> {
        self.file.write_all(packet)
    }

    pub(crate) fn read_exact(&mut self, response: &mut [u8]) -> io::Result<()> {
        self.file.read_exact(response)
    }
}

impl PacketTransport for I2cDevice {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if packet.len() < 2 || packet[0] != DDC_CI_DATA_ADDRESS {
            return Err(TransportError::InvalidRequest(
                "invalid DDC packet passed to the Linux transport",
            ));
        }
        self.write_all(packet)
            .map_err(|error| transport_error("i2c-dev write", error))
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        self.read_exact(response)
            .map_err(|error| transport_error("i2c-dev read", error))
    }
}

pub(crate) fn bus_number(name: &OsStr) -> Option<u32> {
    name.to_str()?.strip_prefix("i2c-")?.parse::<u32>().ok()
}

fn transport_error(operation: &'static str, error: io::Error) -> TransportError {
    if error.kind() == io::ErrorKind::UnexpectedEof {
        TransportError::UnexpectedEof(operation)
    } else {
        TransportError::Io {
            operation,
            code: error.raw_os_error().unwrap_or(-1),
        }
    }
}

fn set_slave_address(file: &File, address: u16) -> io::Result<()> {
    const EBUSY: i32 = 16;

    // SAFETY: the fd is valid, both requests expect an integer address, and no pointer is borrowed.
    let result = unsafe { ioctl(file.as_raw_fd(), I2C_SLAVE, i32::from(address)) };
    if result >= 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() != Some(EBUSY) {
        return Err(error);
    }

    // SAFETY: same valid fd and integer argument as the I2C_SLAVE call above.
    let result = unsafe { ioctl(file.as_raw_fd(), I2C_SLAVE_FORCE, i32::from(address)) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
