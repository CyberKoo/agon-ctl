use crate::framework::{core_display, core_graphics, io_kit};
use agon_osd::{DisplayInfo, PacketTransport, TransportError};
use std::error::Error;
use std::fmt;

const DDC_CI_ADDRESS: u32 = 0x37;
const MCDP29XX_DDC_CI_ADDRESS: u32 = 0xB7;
const DDC_CI_DATA_ADDRESS: u32 = 0x51;

pub struct MacDisplay {
    info: DisplayInfo,
    service: core_display::AvService,
    i2c_address: u32,
}

impl MacDisplay {
    pub fn info(&self) -> &DisplayInfo {
        &self.info
    }

    fn connect(info: DisplayInfo) -> Result<Option<Self>, MacDisplayError> {
        let iterator = io_kit::registry_iterator().map_err(MacDisplayError::Registry)?;
        let mut display_location = None;
        let candidates = iterator.filter_map(|entry| {
            match entry.name().as_deref() {
                Some("AppleCLCD2" | "IOMobileFramebufferShim") => {
                    display_location = entry.path();
                    return None;
                }
                Some("DCPAVServiceProxy")
                    if display_location.as_deref() == Some(info.location()) => {}
                _ => return None,
            }
            if entry.property_string("Location").as_deref() != Some("External") {
                return None;
            }
            let service = core_display::AvService::new(&entry)?;
            let is_mcdp29xx = entry
                .parent()
                .and_then(|parent| parent.property_string("EPICProviderClass"))
                .as_deref()
                == Some("AppleDCPMCDP29XX");
            Some((service, is_mcdp29xx))
        });
        // There is no reliable discriminator between multiple usable services
        // associated with one framebuffer. Refuse the display rather than send
        // commands to the wrong one.
        Ok(
            unique_candidate(candidates).map(|(service, is_mcdp29xx)| Self {
                info,
                service,
                i2c_address: if is_mcdp29xx {
                    MCDP29XX_DDC_CI_ADDRESS
                } else {
                    DDC_CI_ADDRESS
                },
            }),
        )
    }
}

fn unique_candidate<T>(mut candidates: impl Iterator<Item = T>) -> Option<T> {
    let candidate = candidates.next()?;
    if candidates.next().is_some() {
        None
    } else {
        Some(candidate)
    }
}

impl PacketTransport for MacDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if packet.len() < 2 || packet[0] != DDC_CI_DATA_ADDRESS as u8 {
            return Err(TransportError::InvalidRequest(
                "invalid DDC packet passed to the macOS transport",
            ));
        }
        self.service
            .write_i2c(self.i2c_address, DDC_CI_DATA_ADDRESS, &packet[1..])
            .map_err(|code| TransportError::Io {
                operation: "IOAVServiceWriteI2C",
                code,
            })
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        self.service
            .read_i2c(self.i2c_address, 0, response)
            .map_err(|code| TransportError::Io {
                operation: "IOAVServiceReadI2C",
                code,
            })
    }
}

#[derive(Debug)]
pub enum MacDisplayError {
    CoreGraphics(i32),
    Registry(i32),
}

impl fmt::Display for MacDisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreGraphics(code) => write!(formatter, "CoreGraphics failed with error {code}"),
            Self::Registry(code) => write!(formatter, "IOKit registry failed with error {code:#x}"),
        }
    }
}

impl Error for MacDisplayError {}

pub fn enumerate() -> Result<Vec<MacDisplay>, MacDisplayError> {
    let ids =
        core_graphics::online_external_display_ids().map_err(MacDisplayError::CoreGraphics)?;
    let mut displays = Vec::new();
    for id in ids {
        let Some(dictionary) = core_display::display_info(id) else {
            continue;
        };
        let Some(location) = dictionary.string("IODisplayLocation") else {
            continue;
        };
        let vendor_id = core_graphics::vendor_number(id);
        let model_id = core_graphics::model_number(id);
        let name = dictionary
            .nested_string("DisplayProductName", "en_US")
            .unwrap_or_else(|| format!("{vendor_id:04x}:{model_id:04x}"));
        let info = DisplayInfo::new(
            id.to_string(),
            name,
            vendor_id,
            model_id,
            core_graphics::serial_number(id),
            location,
        );
        if let Some(display) = MacDisplay::connect(info)? {
            displays.push(display);
        }
    }
    Ok(displays)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn no_usable_candidate_is_rejected() {
        assert_eq!(unique_candidate(std::iter::empty::<u32>()), None);
    }

    #[test]
    fn single_usable_candidate_is_selected() {
        let candidates = [None, Some((7, true)), None];
        assert_eq!(
            unique_candidate(candidates.into_iter().flatten()),
            Some((7, true))
        );
    }

    #[test]
    fn ambiguous_usable_candidates_are_rejected_and_dropped() {
        struct Candidate<'a>(&'a Cell<usize>);

        impl Drop for Candidate<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let dropped = Cell::new(0);
        let candidates = [
            Candidate(&dropped),
            Candidate(&dropped),
            Candidate(&dropped),
        ];
        assert!(unique_candidate(candidates.into_iter()).is_none());
        assert_eq!(dropped.get(), 3);
    }
}
