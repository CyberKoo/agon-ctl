const EDID_HEADER: &[u8; 8] = b"\x00\xFF\xFF\xFF\xFF\xFF\xFF\x00";
const MONITOR_NAME_TAG: u8 = 0xFC;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EdidIdentity {
    pub name: Option<String>,
    pub vendor_id: u32,
    pub model_id: u32,
    pub serial_number: u32,
}

pub fn parse_edid(edid: &[u8]) -> Option<EdidIdentity> {
    if edid.len() < 128
        || &edid[..8] != EDID_HEADER
        || edid[..128]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
            != 0
    {
        return None;
    }
    Some(EdidIdentity {
        name: monitor_name(edid),
        vendor_id: u32::from(u16::from_be_bytes([edid[8], edid[9]])),
        model_id: u32::from(u16::from_le_bytes([edid[10], edid[11]])),
        serial_number: u32::from_le_bytes([edid[12], edid[13], edid[14], edid[15]]),
    })
}

fn monitor_name(edid: &[u8]) -> Option<String> {
    edid[54..126]
        .chunks_exact(18)
        .filter(|descriptor| descriptor[..3] == [0, 0, 0] && descriptor[3] == MONITOR_NAME_TAG)
        .find_map(|descriptor| {
            let text = String::from_utf8_lossy(&descriptor[5..]);
            let name = text
                .trim_matches(|character: char| {
                    character == '\0' || character.is_ascii_whitespace()
                })
                .to_owned();
            (!name.is_empty()).then_some(name)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_edid() -> [u8; 128] {
        let mut bytes = [0; 128];
        bytes[..8].copy_from_slice(EDID_HEADER);
        bytes[8..16].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 1, 2, 3, 4]);
        bytes[127] = 0_u8.wrapping_sub(bytes[..127].iter().fold(0_u8, |a, b| a.wrapping_add(*b)));
        bytes
    }

    #[test]
    fn valid_base_block_provides_identity() {
        let identity = parse_edid(&valid_edid()).unwrap();
        assert_eq!(identity.vendor_id, 0x1234);
        assert_eq!(identity.model_id, 0x7856);
        assert_eq!(identity.serial_number, 0x04030201);
    }

    #[test]
    fn malformed_or_corrupt_base_block_is_not_an_identity() {
        assert!(parse_edid(&valid_edid()[..127]).is_none());
        let mut corrupt = valid_edid();
        corrupt[12] ^= 1;
        assert!(parse_edid(&corrupt).is_none());
        corrupt = valid_edid();
        corrupt[0] = 1;
        assert!(parse_edid(&corrupt).is_none());
    }

    #[test]
    fn unavailable_extensions_do_not_invalidate_a_valid_base_block() {
        let mut bytes = valid_edid();
        bytes[126] = 1;
        bytes[127] = bytes[127].wrapping_sub(1);
        assert!(parse_edid(&bytes).is_some());
    }
}
