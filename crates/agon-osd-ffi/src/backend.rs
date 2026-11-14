//! Thread-confined device access; backends deliberately have no `Send` bound.
#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", test))]

mod displays;

use crate::error::ApiError;
use agon_osd::{DisplayInfo, Feature, VcpValue};
use displays::Displays;

#[cfg(target_os = "linux")]
use agon_osd_linux::{LinuxDisplay as NativeDisplay, enumerate};
#[cfg(target_os = "macos")]
use agon_osd_macos::{MacDisplay as NativeDisplay, enumerate};
#[cfg(target_os = "windows")]
use agon_osd_windows::{WindowsDisplay as NativeDisplay, enumerate};

pub(crate) trait Backend {
    fn refresh(&mut self) -> Result<Vec<DisplayInfo>, ApiError>;
    fn read(&mut self, id: &str, feature: Feature) -> Result<VcpValue, ApiError>;
    fn set(&mut self, id: &str, feature: Feature, value: u16) -> Result<VcpValue, ApiError>;
    fn probe(&mut self, id: &str, feature: Feature) -> Result<Option<VcpValue>, ApiError>;
    fn capabilities(&mut self, id: &str, feature: Feature) -> Result<Vec<u8>, ApiError>;
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub(crate) struct NativeBackend {
    displays: Displays<NativeDisplay>,
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
impl NativeBackend {
    pub(crate) fn new() -> Result<Self, ApiError> {
        Ok(Self {
            displays: Displays::new(),
        })
    }
}

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
impl Backend for NativeBackend {
    fn refresh(&mut self) -> Result<Vec<DisplayInfo>, ApiError> {
        let displays = enumerate()
            .map_err(|error| ApiError::new(crate::error::TRANSPORT_ERROR, error.to_string()))?;
        self.displays.replace(
            displays
                .into_iter()
                .map(|display| (display.info().clone(), display))
                .collect(),
        );
        Ok(self.displays.infos().to_vec())
    }

    fn read(&mut self, id: &str, feature: Feature) -> Result<VcpValue, ApiError> {
        self.displays.display(id)?.read(feature).map_err(Into::into)
    }

    fn set(&mut self, id: &str, feature: Feature, value: u16) -> Result<VcpValue, ApiError> {
        self.displays
            .display(id)?
            .set(feature, value)
            .map_err(Into::into)
    }

    fn probe(&mut self, id: &str, feature: Feature) -> Result<Option<VcpValue>, ApiError> {
        self.displays
            .display(id)?
            .probe(feature)
            .map_err(Into::into)
    }

    fn capabilities(&mut self, id: &str, feature: Feature) -> Result<Vec<u8>, ApiError> {
        let osd = self.displays.display(id)?;
        let capabilities = osd.read_capabilities()?;
        Ok(capabilities
            .listed_values(feature.definition().address)
            .unwrap_or(&[])
            .to_vec())
    }
}
