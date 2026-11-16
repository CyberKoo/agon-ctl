use crate::display::LinuxDisplay;
use crate::error::LinuxDisplayError;
use crate::i2c::bus_number;
use crate::naming::DisplayNamer;
use crate::probe::{ProbeError, ProbedDevice, probe_device};
use agon_osd::DisplayInfo;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const DRM_SYSFS_PATH: &str = "/sys/class/drm";
const I2C_DEV_PATH: &str = "/dev";

struct DiscoveredDisplay<T> {
    info: DisplayInfo,
    device: T,
}

/// Enumerates displays without emitting recoverable diagnostics.
pub fn enumerate() -> Result<Vec<LinuxDisplay>, LinuxDisplayError> {
    enumerate_with_diagnostics(|_| {})
}

/// Enumerates displays and reports recoverable diagnostics to the caller.
pub fn enumerate_with_diagnostics(
    mut diagnostic: impl FnMut(String),
) -> Result<Vec<LinuxDisplay>, LinuxDisplayError> {
    enumerate_with(
        Path::new(I2C_DEV_PATH),
        Path::new(DRM_SYSFS_PATH),
        probe_device,
        &mut diagnostic,
    )
    .map(|displays| {
        displays
            .into_iter()
            .map(|display| LinuxDisplay::new(display.info, display.device))
            .collect()
    })
}

fn enumerate_with<T>(
    i2c_dev_path: &Path,
    drm_path: &Path,
    mut probe: impl FnMut(&Path) -> Result<ProbedDevice<T>, ProbeError>,
    diagnostic: &mut dyn FnMut(String),
) -> Result<Vec<DiscoveredDisplay<T>>, LinuxDisplayError> {
    let entries = fs::read_dir(i2c_dev_path).map_err(|source| LinuxDisplayError::Io {
        path: i2c_dev_path.to_owned(),
        source,
    })?;
    let mut probed = BTreeMap::new();
    let mut permission_reported = false;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                diagnostic(format!(
                    "cannot read entry in {}: {source}",
                    i2c_dev_path.display()
                ));
                continue;
            }
        };
        let Some(bus) = bus_number(&entry.file_name()) else {
            continue;
        };
        if probed.contains_key(&bus) {
            continue;
        }
        let device_path = entry.path();
        match probe(&device_path) {
            Ok(display) => {
                probed.insert(bus, (device_path, display));
            }
            Err(ProbeError::NoDisplay) => {}
            Err(ProbeError::PermissionDenied) => {
                if !permission_reported {
                    diagnostic(format!(
                        "cannot access {}: permission denied; add the current user to the i2c group and log in again",
                        i2c_dev_path.join("i2c-*").display()
                    ));
                    permission_reported = true;
                }
            }
            Err(ProbeError::InvalidResponse(error)) => {
                diagnostic(format!(
                    "{} responded but does not match the supported AOC DDC protocol: {error}",
                    device_path.display()
                ));
            }
        }
    }

    let mut namer = DisplayNamer::load(drm_path, diagnostic);
    Ok(probed
        .into_iter()
        .map(|(bus, (device_path, probed))| {
            let info = namer.display_info(bus, &device_path, probed.identity);
            DiscoveredDisplay {
                info,
                device: probed.device,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Fixture;
    use agon_osd::{DdcError, EdidIdentity};

    fn bus_from_path(path: &Path) -> u32 {
        bus_number(path.file_name().unwrap()).unwrap()
    }

    fn probed(bus: u32, identity: Option<EdidIdentity>) -> Result<ProbedDevice<u32>, ProbeError> {
        Ok(ProbedDevice {
            device: bus,
            identity,
        })
    }

    #[test]
    fn enumeration_does_not_require_connector_ddc_nodes() {
        let fixture = Fixture::new();
        for bus in [4, 7, 8] {
            fixture.bus(bus);
        }
        fs::write(fixture.i2c.join("not-i2c"), []).unwrap();
        fixture.connector("card2-DP-4", "connected", None, None);

        let mut diagnostics = Vec::new();
        let displays = enumerate_with(
            &fixture.i2c,
            &fixture.drm,
            |path| match bus_from_path(path) {
                bus @ (4 | 7) => probed(bus, None),
                _ => Err(ProbeError::NoDisplay),
            },
            &mut |warning| diagnostics.push(warning),
        )
        .unwrap();

        let ids: Vec<_> = displays.iter().map(|display| display.info.id()).collect();
        assert_eq!(ids, ["i2c-4", "i2c-7"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn permission_errors_are_reported_once_and_other_buses_are_retained() {
        let fixture = Fixture::new();
        for bus in 1..=4 {
            fixture.bus(bus);
        }
        let mut diagnostics = Vec::new();
        let displays = enumerate_with(
            &fixture.i2c,
            &fixture.drm,
            |path| match bus_from_path(path) {
                1 | 2 => Err(ProbeError::PermissionDenied),
                bus @ 3 => probed(bus, None),
                _ => Err(ProbeError::NoDisplay),
            },
            &mut |warning| diagnostics.push(warning),
        )
        .unwrap();

        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].info.id(), "i2c-3");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("permission denied"));
        assert!(diagnostics[0].contains("i2c group"));
    }

    #[test]
    fn invalid_aoc_responses_are_reported_without_exposing_no_response_buses() {
        let fixture = Fixture::new();
        for bus in [4, 5, 6] {
            fixture.bus(bus);
        }
        let mut diagnostics = Vec::new();
        let displays = enumerate_with(
            &fixture.i2c,
            &fixture.drm,
            |path| match bus_from_path(path) {
                4 => Err(ProbeError::NoDisplay),
                5 => Err(ProbeError::InvalidResponse(DdcError::ChecksumMismatch)),
                bus => probed(bus, None),
            },
            &mut |warning| diagnostics.push(warning),
        )
        .unwrap();

        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].info.id(), "i2c-6");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("i2c-5"));
        assert!(diagnostics[0].contains("supported AOC DDC protocol"));
    }

    #[test]
    fn unreadable_i2c_root_is_an_error() {
        let fixture = Fixture::new();
        let result = enumerate_with(
            &fixture.i2c.join("missing"),
            &fixture.drm,
            |path| probed(bus_from_path(path), None),
            &mut |_| {},
        );
        assert!(matches!(result, Err(LinuxDisplayError::Io { .. })));
    }

    #[test]
    fn unreadable_drm_root_only_loses_optional_names() {
        let fixture = Fixture::new();
        fixture.bus(4);
        let mut diagnostics = Vec::new();
        let displays = enumerate_with(
            &fixture.i2c,
            &fixture.drm.join("missing"),
            |path| probed(bus_from_path(path), None),
            &mut |warning| diagnostics.push(warning),
        )
        .unwrap();

        assert_eq!(displays.len(), 1);
        assert_eq!(displays[0].info.name(), "i2c-4");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("display names"));
    }
}
