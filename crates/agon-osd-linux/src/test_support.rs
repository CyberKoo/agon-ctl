use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) const EDID_BASE_BLOCK_SIZE: usize = 128;

pub(crate) struct Fixture {
    root: PathBuf,
    pub(crate) i2c: PathBuf,
    pub(crate) drm: PathBuf,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "agon-linux-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let i2c = root.join("dev");
        let drm = root.join("drm");
        fs::create_dir_all(&i2c).unwrap();
        fs::create_dir_all(&drm).unwrap();
        Self { root, i2c, drm }
    }

    pub(crate) fn bus(&self, bus: u32) {
        fs::write(self.i2c.join(format!("i2c-{bus}")), []).unwrap();
    }

    pub(crate) fn connector(
        &self,
        name: &str,
        status: &str,
        bus: Option<u32>,
        edid: Option<&[u8; EDID_BASE_BLOCK_SIZE]>,
    ) -> PathBuf {
        let connector = self.drm.join(name);
        fs::create_dir(&connector).unwrap();
        fs::write(connector.join("status"), status).unwrap();
        if let Some(bus) = bus {
            fs::create_dir_all(
                connector
                    .join("ddc")
                    .join("i2c-dev")
                    .join(format!("i2c-{bus}")),
            )
            .unwrap();
        }
        if let Some(edid) = edid {
            fs::write(connector.join("edid"), edid).unwrap();
        }
        connector
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

pub(crate) fn valid_edid(
    name: &str,
    vendor_id: u16,
    model_id: u16,
    serial_number: u32,
) -> [u8; EDID_BASE_BLOCK_SIZE] {
    let mut bytes = [0; EDID_BASE_BLOCK_SIZE];
    bytes[..8].copy_from_slice(b"\x00\xFF\xFF\xFF\xFF\xFF\xFF\x00");
    bytes[8..10].copy_from_slice(&vendor_id.to_be_bytes());
    bytes[10..12].copy_from_slice(&model_id.to_le_bytes());
    bytes[12..16].copy_from_slice(&serial_number.to_le_bytes());
    bytes[54..59].copy_from_slice(&[0, 0, 0, 0xFC, 0]);
    bytes[59..59 + name.len()].copy_from_slice(name.as_bytes());
    bytes[127] = 0_u8.wrapping_sub(
        bytes[..127]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte)),
    );
    bytes
}
