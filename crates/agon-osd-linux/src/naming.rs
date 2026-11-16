use crate::i2c::bus_number;
use agon_osd::{DisplayInfo, EdidIdentity, parse_edid};
use std::fs;
use std::path::Path;

pub(crate) struct DisplayNamer {
    connectors: Vec<Option<Connector>>,
}

impl DisplayNamer {
    pub(crate) fn load(drm_path: &Path, diagnostic: &mut dyn FnMut(String)) -> Self {
        Self {
            connectors: read_connectors(drm_path, diagnostic),
        }
    }

    pub(crate) fn display_info(
        &mut self,
        bus: u32,
        device_path: &Path,
        ddc_identity: Option<EdidIdentity>,
    ) -> DisplayInfo {
        let connector = self.take_matching_connector(bus, ddc_identity.as_ref());
        let id = format!("i2c-{bus}");
        let (name, identity) = match connector {
            Some(connector) => {
                let identity = connector.identity.or(ddc_identity).unwrap_or_default();
                let name = identity.name.clone().unwrap_or(connector.name);
                (name, identity)
            }
            None => (id.clone(), EdidIdentity::default()),
        };
        DisplayInfo::new(
            id,
            name,
            identity.vendor_id,
            identity.model_id,
            identity.serial_number,
            device_path.to_string_lossy().into_owned(),
        )
    }

    fn take_matching_connector(
        &mut self,
        bus: u32,
        identity: Option<&EdidIdentity>,
    ) -> Option<Connector> {
        if let Some(index) = self.connectors.iter().position(|connector| {
            connector
                .as_ref()
                .is_some_and(|connector| connector.bus == Some(bus))
        }) {
            return self.connectors[index].take();
        }

        let identity = identity?;
        let mut matches = self
            .connectors
            .iter()
            .enumerate()
            .filter_map(|(index, connector)| {
                let candidate = connector.as_ref()?.identity.as_ref()?;
                (candidate.vendor_id == identity.vendor_id
                    && candidate.model_id == identity.model_id
                    && candidate.serial_number == identity.serial_number)
                    .then_some(index)
            });
        let index = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        self.connectors[index].take()
    }
}

struct Connector {
    name: String,
    identity: Option<EdidIdentity>,
    bus: Option<u32>,
}

fn read_connectors(drm_path: &Path, diagnostic: &mut dyn FnMut(String)) -> Vec<Option<Connector>> {
    let entries = match fs::read_dir(drm_path) {
        Ok(entries) => entries,
        Err(source) => {
            diagnostic(format!(
                "cannot read {}: {source}; display names will use i2c bus IDs",
                drm_path.display()
            ));
            return Vec::new();
        }
    };
    let mut connectors = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                diagnostic(format!(
                    "cannot read entry in {}: {source}",
                    drm_path.display()
                ));
                continue;
            }
        };
        let path = entry.path();
        let Some(name) = connector_name(&path) else {
            continue;
        };
        if is_internal_connector(name) {
            continue;
        }
        let status_path = path.join("status");
        match fs::read_to_string(&status_path) {
            Ok(status) if status.trim() == "connected" => {}
            Ok(_) => continue,
            Err(source) => {
                diagnostic(format!(
                    "cannot read {}: {source}; connector will not be used for naming",
                    status_path.display()
                ));
                continue;
            }
        }
        let identity = fs::read(path.join("edid"))
            .ok()
            .and_then(|bytes| parse_edid(&bytes));
        connectors.push(Some(Connector {
            name: entry.file_name().to_string_lossy().into_owned(),
            identity,
            bus: direct_connector_bus(&path),
        }));
    }
    connectors
}

fn direct_connector_bus(connector: &Path) -> Option<u32> {
    let entries = fs::read_dir(connector.join("ddc").join("i2c-dev")).ok()?;
    entries
        .filter_map(Result::ok)
        .find_map(|entry| bus_number(&entry.file_name()))
}

/// Recognize cardN-TYPE-N (including compound types and DP MST suffixes).
fn connector_name(path: &Path) -> Option<&str> {
    let name = path.file_name()?.to_str()?;
    let (card, connector) = name.strip_prefix("card")?.split_once('-')?;
    let (kind, index) = connector.rsplit_once('-')?;
    let is_number = |value: &str| !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    if !is_number(card)
        || !is_number(index)
        || !kind.as_bytes().first()?.is_ascii_alphabetic()
        || !kind
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric()))
    {
        return None;
    }
    Some(connector)
}

fn is_internal_connector(name: &str) -> bool {
    matches!(name.split('-').next(), Some("eDP" | "LVDS" | "DSI"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{Fixture, valid_edid};

    #[test]
    fn connector_names_require_card_number_type_and_index() {
        for name in [
            "card0-DP-1",
            "card1-HDMI-A-1",
            "card12-DVI-D-2",
            "card0-DP-1-2",
            "card0-eDP-1",
            "card1-LVDS-1",
            "card2-DSI-1",
        ] {
            assert!(connector_name(Path::new(name)).is_some(), "{name}");
        }
        for name in [
            "card0",
            "renderD128",
            "not-a-connector",
            "card-DP-1",
            "cardx-DP-1",
            "card0-DP",
            "card0-DP-",
            "card0-DP-x",
            "card0--1",
            "card0-DP--1",
            "card0-1-2",
            "card0-DP_foo-1",
            "other0-DP-1",
        ] {
            assert!(connector_name(Path::new(name)).is_none(), "{name}");
        }
    }

    #[test]
    fn only_explicit_internal_connector_types_are_excluded() {
        for name in ["eDP-1", "LVDS-1", "DSI-2"] {
            assert!(is_internal_connector(name));
        }
        for name in ["DP-1", "HDMI-A-1", "DVI-D-1", "Virtual-1", "eDPextra-1"] {
            assert!(!is_internal_connector(name));
        }
    }

    #[test]
    fn direct_i2c_dev_directory_provides_a_name_without_ddc_edid() {
        let fixture = Fixture::new();
        let edid = valid_edid("AGON DIRECT", 0x1234, 0x5678, 42);
        fixture.connector("card1-DP-1", "connected\n", Some(5), Some(&edid));
        let mut namer = DisplayNamer::load(&fixture.drm, &mut |_| {});

        let info = namer.display_info(5, Path::new("/dev/i2c-5"), None);
        assert_eq!(info.name(), "AGON DIRECT");
    }

    #[test]
    fn unique_edid_identity_matches_a_connector() {
        let fixture = Fixture::new();
        let edid = valid_edid("AGON TEST", 0x1234, 0x5678, 42);
        fixture.connector("card2-DP-4", "connected", None, Some(&edid));
        let mut namer = DisplayNamer::load(&fixture.drm, &mut |_| {});

        let info = namer.display_info(4, Path::new("/dev/i2c-4"), parse_edid(&edid));
        assert_eq!(info.name(), "AGON TEST");
        assert_eq!(info.vendor_id(), 0x1234);
        assert_eq!(info.model_id(), 0x5678);
        assert_eq!(info.serial_number(), 42);
    }

    #[test]
    fn ambiguous_or_ineligible_edid_matches_fall_back_to_bus_ids() {
        let fixture = Fixture::new();
        let edid = valid_edid("AGON DUPLICATE", 0x1234, 0x5678, 42);
        fixture.connector("card0-DP-1", "connected", None, Some(&edid));
        fixture.connector("card0-DP-2", "connected", None, Some(&edid));
        fixture.connector("card0-eDP-1", "connected", None, Some(&edid));
        fixture.connector("card0-DP-3", "disconnected", None, Some(&edid));
        let mut namer = DisplayNamer::load(&fixture.drm, &mut |_| {});

        let ambiguous = namer.display_info(4, Path::new("/dev/i2c-4"), parse_edid(&edid));
        let unmatched = namer.display_info(5, Path::new("/dev/i2c-5"), None);
        assert_eq!(ambiguous.name(), "i2c-4");
        assert_eq!(unmatched.name(), "i2c-5");
    }
}
