use crate::i2c::I2cDevice;
use agon_osd::{DisplayInfo, PacketTransport, TransportError};

pub struct LinuxDisplay {
    info: DisplayInfo,
    device: I2cDevice,
}

impl LinuxDisplay {
    pub(crate) fn new(info: DisplayInfo, device: I2cDevice) -> Self {
        Self { info, device }
    }

    pub fn info(&self) -> &DisplayInfo {
        &self.info
    }
}

impl PacketTransport for LinuxDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        self.device.write_packet(packet)
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        self.device.read_packet(response)
    }
}
