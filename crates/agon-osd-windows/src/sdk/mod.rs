pub(crate) mod adl;
pub(crate) mod igcl;
pub(crate) mod nvapi;

#[cfg(all(test, target_os = "windows"))]
mod tests;

use agon_osd::{DisplayInfo, PacketTransport, TransportError};

pub(crate) struct DiscoveredDisplay {
    pub(crate) info: DisplayInfo,
    pub(crate) edid: [u8; 128],
    pub(crate) raw: RawDisplay,
}

pub(crate) enum RawDisplay {
    Intel(igcl::IgclDisplay),
    Amd(adl::AdlDisplay),
    Nvidia(nvapi::NvDisplay),
}

impl PacketTransport for RawDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        match self {
            Self::Intel(display) => display.write_packet(packet),
            Self::Amd(display) => display.write_packet(packet),
            Self::Nvidia(display) => display.write_packet(packet),
        }
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        match self {
            Self::Intel(display) => display.read_packet(response),
            Self::Amd(display) => display.read_packet(response),
            Self::Nvidia(display) => display.read_packet(response),
        }
    }
}
