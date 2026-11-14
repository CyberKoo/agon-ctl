use crate::i2c::I2cDevice;
use agon_osd::{DisplayInfo, PacketTransport, TransportError, parse_edid};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DRM_SYSFS_PATH: &str = "/sys/class/drm";
const DDC_CI_ADDRESS: u16 = 0x37;
const DDC_CI_DATA_ADDRESS: u8 = 0x51;

pub struct LinuxDisplay {
    info: DisplayInfo,
    device: I2cDevice,
}

impl LinuxDisplay {
    pub fn info(&self) -> &DisplayInfo {
        &self.info
    }
}

impl PacketTransport for LinuxDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if packet.len() < 2 || packet[0] != DDC_CI_DATA_ADDRESS {
            return Err(TransportError::InvalidRequest(
                "invalid DDC packet passed to the Linux transport",
            ));
        }
        self.device
            .write_packet(packet)
            .map_err(|error| transport_error("i2c-dev write", error))
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        self.device
            .read_packet(response)
            .map_err(|error| transport_error("i2c-dev read", error))
    }
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

#[derive(Debug)]
pub enum LinuxDisplayError {
    Io { path: PathBuf, source: io::Error },
    InvalidDdcPath(PathBuf),
}

impl fmt::Display for LinuxDisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::InvalidDdcPath(path) => {
                write!(formatter, "invalid DRM DDC path: {}", path.display())
            }
        }
    }
}

impl Error for LinuxDisplayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidDdcPath(_) => None,
        }
    }
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
        Path::new(DRM_SYSFS_PATH),
        |info, device_path| {
            I2cDevice::open(device_path, DDC_CI_ADDRESS).map(|device| LinuxDisplay { info, device })
        },
        &mut diagnostic,
    )
}

fn enumerate_with<T>(
    drm_path: &Path,
    mut open: impl FnMut(DisplayInfo, &Path) -> io::Result<T>,
    diagnostic: &mut dyn FnMut(String),
) -> Result<Vec<T>, LinuxDisplayError> {
    let entries = fs::read_dir(drm_path).map_err(|source| LinuxDisplayError::Io {
        path: drm_path.to_owned(),
        source,
    })?;
    let mut displays = BTreeMap::new();
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
        let connector = entry.path();
        let Some(name) = connector_name(&connector) else {
            continue;
        };
        if is_internal_connector(name) {
            continue;
        }
        let bus = match connector_bus(&connector) {
            Ok(Some(bus)) => bus,
            Ok(None) => continue,
            Err(error) => {
                diagnostic(format!(
                    "skipping connector {}: {error}",
                    connector.display()
                ));
                continue;
            }
        };
        if displays.contains_key(&bus) {
            continue;
        }

        let device_path = PathBuf::from(format!("/dev/i2c-{bus}"));
        let identity = fs::read(connector.join("edid"))
            .ok()
            .and_then(|bytes| parse_edid(&bytes))
            .unwrap_or_default();
        let id = format!("i2c-{bus}");
        let name = identity.name.unwrap_or_else(|| {
            connector
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| id.clone())
        });
        let info = DisplayInfo::new(
            id,
            name,
            identity.vendor_id,
            identity.model_id,
            identity.serial_number,
            device_path.to_string_lossy().into_owned(),
        );
        match open(info, &device_path) {
            Ok(display) => {
                displays.insert(bus, display);
            }
            Err(error) => {
                diagnostic(format!(
                    "skipping connector {}: cannot open {}: {error}",
                    connector.display(),
                    device_path.display()
                ));
            }
        }
    }
    Ok(displays.into_values().collect())
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

fn connector_bus(connector: &Path) -> Result<Option<u32>, LinuxDisplayError> {
    let status_path = connector.join("status");
    let status = match fs::read_to_string(&status_path) {
        Ok(status) => status,
        Err(source) => {
            return Err(LinuxDisplayError::Io {
                path: status_path,
                source,
            });
        }
    };
    if status.trim() != "connected" {
        return Ok(None);
    }

    let ddc_path = connector.join("ddc");
    let resolved = match fs::canonicalize(&ddc_path) {
        Ok(path) => path,
        Err(source) => {
            return Err(LinuxDisplayError::Io {
                path: ddc_path,
                source,
            });
        }
    };
    let bus = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("i2c-"))
        .and_then(|number| number.parse::<u32>().ok())
        .ok_or(LinuxDisplayError::InvalidDdcPath(resolved))?;
    Ok(Some(bus))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "agon-linux-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn connector(&self, name: &str, status: &str, bus: u32) -> PathBuf {
            let connector = self.0.join(name);
            fs::create_dir(&connector).unwrap();
            fs::write(connector.join("status"), status).unwrap();
            let target = self.0.join(format!("i2c-{bus}"));
            fs::create_dir_all(&target).unwrap();
            symlink(target, connector.join("ddc")).unwrap();
            connector
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

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
    fn enumeration_keeps_external_displays_and_existing_bus_ids() {
        let fixture = Fixture::new();
        fixture.connector("card0-eDP-1", "connected", 1);
        fixture.connector("card0-LVDS-1", "connected", 2);
        fixture.connector("card0-DSI-1", "connected", 3);
        fixture.connector("card0-HDMI-A-1", "connected\n", 7);
        fixture.connector("card1-DP-1", "connected", 4);
        fixture.connector("card1-DP-2", "connected", 4);
        fixture.connector("card0-DP-3", "disconnected", 8);
        fixture.connector("not-a-connector", "connected", 9);
        fixture.connector("cardx-DP-1", "connected", 10);
        fixture.connector("card0-DP--1", "connected", 11);

        let displays = enumerate_with(
            &fixture.0,
            |info, path| {
                assert!(matches!(path.to_str(), Some("/dev/i2c-4" | "/dev/i2c-7")));
                Ok(info)
            },
            &mut |_| {},
        )
        .unwrap();
        let ids: Vec<_> = displays.iter().map(|info| info.id()).collect();
        assert_eq!(ids, ["i2c-4", "i2c-7"]);
    }

    #[test]
    fn bad_status_ddc_and_device_open_do_not_discard_other_displays() {
        let fixture = Fixture::new();
        fixture.connector("card0-DP-1", "connected", 1);
        let missing = fixture.connector("card0-DP-2", "connected", 2);
        fs::remove_file(missing.join("status")).unwrap();
        let unreadable = fixture.connector("card0-DP-3", "connected", 3);
        fs::remove_file(unreadable.join("status")).unwrap();
        fs::create_dir(unreadable.join("status")).unwrap();
        let missing_ddc = fixture.connector("card0-DP-4", "connected", 4);
        fs::remove_dir(fixture.0.join("i2c-4")).unwrap();
        assert!(connector_bus(&missing_ddc).is_err());
        let invalid = fixture.connector("card0-DP-5", "connected", 5);
        fs::remove_file(invalid.join("ddc")).unwrap();
        symlink(&fixture.0, invalid.join("ddc")).unwrap();
        assert!(matches!(
            connector_bus(&invalid),
            Err(LinuxDisplayError::InvalidDdcPath(_))
        ));
        fixture.connector("card0-DP-6", "connected", 6);
        fixture.connector("card0-DP-7", "connected", 7);
        fixture.connector("card0-HDMI-A-1", "connected", 8);

        let mut opened = Vec::new();
        let mut diagnostics = Vec::new();
        let displays = enumerate_with(
            &fixture.0,
            |info, path| {
                opened.push(path.to_owned());
                match path.to_str().unwrap() {
                    "/dev/i2c-6" => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
                    "/dev/i2c-7" => Err(io::Error::from(io::ErrorKind::NotFound)),
                    _ => Ok(info),
                }
            },
            &mut |warning| diagnostics.push(warning),
        )
        .unwrap();
        opened.sort();
        assert_eq!(
            opened,
            [1, 6, 7, 8].map(|bus| PathBuf::from(format!("/dev/i2c-{bus}")))
        );
        let ids: Vec<_> = displays.iter().map(|info| info.id()).collect();
        assert_eq!(ids, ["i2c-1", "i2c-8"]);
        assert_eq!(diagnostics.len(), 6);
        assert!(
            diagnostics
                .iter()
                .all(|warning| warning.starts_with("skipping connector "))
        );
    }

    #[test]
    fn unreadable_sysfs_root_is_still_an_error() {
        let fixture = Fixture::new();
        let result = enumerate_with(&fixture.0.join("missing"), |info, _| Ok(info), &mut |_| {});
        assert!(matches!(result, Err(LinuxDisplayError::Io { .. })));
    }
}
