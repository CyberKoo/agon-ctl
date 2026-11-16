use crate::i2c::I2cDevice;
use agon_osd::{EdidIdentity, parse_edid};
use std::io;
use std::path::Path;

const EDID_ADDRESS: u16 = 0x50;
const EDID_BASE_BLOCK_SIZE: usize = 128;

pub(crate) fn read_identity(path: &Path) -> Option<EdidIdentity> {
    read_base_block(path)
        .ok()
        .and_then(|bytes| parse_edid(&bytes))
}

fn read_base_block(path: &Path) -> io::Result<[u8; EDID_BASE_BLOCK_SIZE]> {
    let mut device = I2cDevice::open(path, EDID_ADDRESS)?;
    device.write_all(&[0])?;
    let mut edid = [0; EDID_BASE_BLOCK_SIZE];
    device.read_exact(&mut edid)?;
    Ok(edid)
}
