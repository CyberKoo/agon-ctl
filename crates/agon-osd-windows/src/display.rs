use crate::dxva2;
use crate::identity::{Identity, Provider};
use crate::routing::Channels;
use crate::sdk::{DiscoveredDisplay, RawDisplay};
use agon_osd::{Address, DdcError, DdcTransport, DisplayInfo, VcpValue};

pub struct WindowsDisplay {
    info: DisplayInfo,
    edid: Option<[u8; 128]>,
    provider: Provider,
    channels: Channels<dxva2::Dxva2Display, RawDisplay>,
}

impl WindowsDisplay {
    pub fn info(&self) -> &DisplayInfo {
        &self.info
    }

    pub(crate) fn from_standard(display: dxva2::Dxva2Display) -> Self {
        Self {
            info: display.info().clone(),
            edid: display.edid(),
            provider: Provider::Dxva2,
            channels: Channels::new(Some(display), Vec::new()),
        }
    }

    pub(crate) fn from_raw(display: DiscoveredDisplay) -> Self {
        let provider = match &display.raw {
            RawDisplay::Intel(_) => Provider::Intel,
            RawDisplay::Amd(_) => Provider::Amd,
            RawDisplay::Nvidia(_) => Provider::Nvidia,
        };
        Self {
            info: display.info,
            edid: Some(display.edid),
            provider,
            channels: Channels::new(None, vec![display.raw]),
        }
    }

    pub(crate) fn identity(&self) -> Identity<'_> {
        Identity {
            provider: self.provider,
            info: &self.info,
            edid: self.edid.as_ref(),
        }
    }

    pub(crate) fn merge(&mut self, display: Self) {
        self.channels.merge(display.channels);
        if self.edid.is_none() {
            self.edid = display.edid;
        }
    }
}

impl DdcTransport for WindowsDisplay {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        self.channels.read_vcp(address)
    }

    fn write_vcp(&mut self, address: Address, value: u16) -> Result<(), DdcError> {
        self.channels.write_vcp(address, value)
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<agon_osd::CapabilityFragment, DdcError> {
        self.channels.read_capabilities(offset)
    }

    fn recover_read(&mut self, address: Address, error: &DdcError) -> bool {
        self.channels.recover_read(address, error)
    }

    fn recover_capabilities(&mut self, error: &DdcError) -> bool {
        self.channels.recover_capabilities(error)
    }
}
