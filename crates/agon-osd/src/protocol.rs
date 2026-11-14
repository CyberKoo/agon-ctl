use crate::{Address, DdcError, VcpValue};

const DDC_SOURCE_ADDRESS: u8 = 0x51;
const DDC_DESTINATION_ADDRESS: u8 = 0x6E;
const DDC_REPLY_ADDRESS: u8 = 0x6E;
const VCP_REPLY_LENGTH: u8 = 8;
const CAPABILITIES_REPLY_OPCODE: u8 = 0xE3;
const CAPABILITIES_MIN_REPLY_LENGTH: u8 = 3;
const CAPABILITIES_MAX_REPLY_LENGTH: u8 = 35;
pub(crate) const VCP_RESPONSE_SIZE: usize = 11;
pub(crate) const CAPABILITIES_RESPONSE_SIZE: usize = 38;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityFragment {
    pub offset: u16,
    pub data: Vec<u8>,
}

pub(crate) struct RequestPacket {
    bytes: [u8; 9],
    len: usize,
}

impl RequestPacket {
    pub(crate) fn read(address: Address) -> Self {
        let mut body = [0; 6];
        let body_len = address.encode_read(&mut body);
        Self::new(body, body_len)
    }

    pub(crate) fn write(address: Address, value: u16) -> Self {
        let mut body = [0; 6];
        let body_len = address.encode_write(value, &mut body);
        Self::new(body, body_len)
    }

    pub(crate) fn capabilities(offset: u16) -> Self {
        let mut body = [0; 6];
        body[0] = 0xF3;
        body[1..3].copy_from_slice(&offset.to_be_bytes());
        Self::new(body, 3)
    }

    fn new(body: [u8; 6], body_len: usize) -> Self {
        let mut bytes = [0; 9];
        bytes[0] = DDC_SOURCE_ADDRESS;
        bytes[1] = 0x80 | body_len as u8;
        bytes[2..2 + body_len].copy_from_slice(&body[..body_len]);
        let checksum_index = 2 + body_len;
        bytes[checksum_index] = bytes[..checksum_index]
            .iter()
            .fold(DDC_DESTINATION_ADDRESS, |checksum, byte| checksum ^ byte);
        Self {
            bytes,
            len: checksum_index + 1,
        }
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

pub(crate) fn decode_response(
    address: Address,
    response: &[u8; VCP_RESPONSE_SIZE],
) -> Result<VcpValue, DdcError> {
    if response[0] != DDC_REPLY_ADDRESS {
        return Err(DdcError::InvalidResponse(
            "invalid DDC response source address",
        ));
    }
    if response[1] & 0x80 == 0 {
        return Err(DdcError::InvalidResponse(
            "invalid DDC response length flag",
        ));
    }
    let payload_length = response[1] & 0x7F;
    if payload_length != VCP_REPLY_LENGTH {
        return Err(DdcError::InvalidResponse("unexpected VCP response length"));
    }

    let checksum = response
        .iter()
        .fold(0x50_u8, |checksum, byte| checksum ^ byte);
    if checksum != 0 {
        return Err(DdcError::ChecksumMismatch);
    }
    if response[2] != 0x02 {
        return Err(DdcError::InvalidResponse("unexpected VCP response opcode"));
    }
    if response[3] != 0 {
        return Err(DdcError::Unsupported {
            address,
            result_code: response[3],
        });
    }

    // Extended replies do not have a verified feature-code echo contract.
    if let Address::Standard(code) = address {
        if response[4] != code {
            return Err(DdcError::InvalidResponse(
                "VCP response feature code mismatch",
            ));
        }
    }

    Ok(VcpValue {
        maximum: u16::from_be_bytes([response[6], response[7]]),
        current: u16::from_be_bytes([response[8], response[9]]),
    })
}

pub(crate) fn decode_capabilities_response(
    expected_offset: u16,
    response: &[u8; CAPABILITIES_RESPONSE_SIZE],
) -> Result<CapabilityFragment, DdcError> {
    if response[0] != DDC_REPLY_ADDRESS {
        return Err(DdcError::InvalidResponse(
            "invalid DDC response source address",
        ));
    }
    if response[1] & 0x80 == 0 {
        return Err(DdcError::InvalidResponse(
            "invalid DDC response length flag",
        ));
    }
    let payload_length = response[1] & 0x7F;
    if !(CAPABILITIES_MIN_REPLY_LENGTH..=CAPABILITIES_MAX_REPLY_LENGTH).contains(&payload_length) {
        return Err(DdcError::InvalidResponse(
            "unexpected capabilities response length",
        ));
    }
    let packet_length = usize::from(payload_length) + 3;
    if response[..packet_length]
        .iter()
        .fold(0x50_u8, |checksum, byte| checksum ^ byte)
        != 0
    {
        return Err(DdcError::ChecksumMismatch);
    }
    if response[2] != CAPABILITIES_REPLY_OPCODE {
        return Err(DdcError::InvalidResponse(
            "unexpected capabilities response opcode",
        ));
    }
    let offset = u16::from_be_bytes([response[3], response[4]]);
    if offset != expected_offset {
        return Err(DdcError::InvalidResponse(
            "capabilities fragment offset mismatch",
        ));
    }
    Ok(CapabilityFragment {
        offset,
        data: response[5..usize::from(payload_length) + 2].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard read frames encode the VCP address in the request body and append the checksum.
    #[test]
    fn read_frame_matches_the_g_menu_judge_probe() {
        let packet = RequestPacket::read(Address::Standard(0x14));
        assert_eq!(packet.as_slice(), &[0x51, 0x82, 0x01, 0x14, 0xA8]);
    }

    /// Write checksums start at 0x6E and exclude the checksum byte itself.
    #[test]
    fn write_checksum_excludes_the_checksum_byte() {
        let packet = RequestPacket::write(Address::Standard(0x10), 0x0064);
        let bytes = packet.as_slice();
        assert_eq!(bytes[0], 0x51, "源地址字节");
        assert_eq!(bytes[1], 0x84, "长度字节 = 正文长度 | 0x80");
        assert_eq!(&bytes[2..6], &[0x03, 0x10, 0x00, 0x64], "正文");
        // 0x6E ^ 0x51 ^ 0x84 ^ 0x03 ^ 0x10 ^ 0x00 ^ 0x64
        assert_eq!(bytes[6], 0xCC, "校验字节");
        let expected = bytes[..6]
            .iter()
            .fold(0x6E, |checksum, byte| checksum ^ byte);
        assert_eq!(bytes[6], expected);
    }

    /// Frame lengths for the four body classes are standard read 2, standard write 4,
    /// extended read 4, and extended write 6. A frame consists of the source address,
    /// length byte, body, and checksum byte, for a total of body length + 3.
    #[test]
    fn frame_lengths_match_the_four_body_classes() {
        assert_eq!(
            RequestPacket::read(Address::Standard(0x10))
                .as_slice()
                .len(),
            5
        );
        assert_eq!(
            RequestPacket::write(Address::Standard(0x10), 1)
                .as_slice()
                .len(),
            7
        );
        assert_eq!(RequestPacket::read(Address::e2a0(0x20)).as_slice().len(), 7);
        assert_eq!(
            RequestPacket::write(Address::e2a0(0x20), 1)
                .as_slice()
                .len(),
            9
        );
    }

    /// Extended-address write frames carry the extended prefix in the body.
    #[test]
    fn extended_write_frame_carries_the_prefix() {
        let packet = RequestPacket::write(Address::f8e1(0x12), 0x0002);
        let bytes = packet.as_slice();
        assert_eq!(bytes[1], 0x86, "长度字节 = 6 | 0x80");
        assert_eq!(&bytes[2..8], &[0x03, 0xF8, 0xE1, 0x12, 0x00, 0x02]);
    }

    /// A valid response starts with 0x6E and sets 0x80 in its length byte. XORing
    /// all 11 bytes from an initial value of 0x50, including the checksum byte,
    /// yields 0.
    #[test]
    fn valid_response_decodes_maximum_and_current() {
        let response = [
            0x6E, 0x88, 0x02, 0x00, 0x10, 0x00, 0x00, 0x64, 0x00, 0x32, 0xF2,
        ];
        assert_eq!(
            response
                .iter()
                .fold(0x50_u8, |checksum, byte| checksum ^ byte),
            0,
            "回包校验必须含校验字节自身"
        );
        let value = decode_response(Address::Standard(0x10), &response).unwrap();
        assert_eq!(value.maximum, 100);
        assert_eq!(value.current, 50);
    }

    #[test]
    fn response_with_wrong_source_address_is_rejected() {
        let mut response = [
            0x6E, 0x88, 0x02, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x32, 0xE2,
        ];
        response[0] = 0x51;
        let error = decode_response(Address::Standard(0x10), &response).unwrap_err();
        assert_eq!(
            error,
            DdcError::InvalidResponse("invalid DDC response source address")
        );
    }

    #[test]
    fn response_without_length_flag_is_rejected() {
        let mut response = [
            0x6E, 0x88, 0x02, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x32, 0xE2,
        ];
        response[1] = 0x08;
        let error = decode_response(Address::Standard(0x10), &response).unwrap_err();
        assert_eq!(
            error,
            DdcError::InvalidResponse("invalid DDC response length flag")
        );
    }

    #[test]
    fn response_with_broken_checksum_is_rejected() {
        let mut response = [
            0x6E, 0x88, 0x02, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x32, 0xE2,
        ];
        response[10] ^= 0xFF;
        let error = decode_response(Address::Standard(0x10), &response).unwrap_err();
        assert_eq!(error, DdcError::ChecksumMismatch);
    }

    #[test]
    fn stale_response_for_another_feature_is_rejected() {
        let response = [
            0x6E, 0x88, 0x02, 0x00, 0x12, 0x00, 0x00, 0x64, 0x00, 0x32, 0xF0,
        ];
        assert_eq!(
            decode_response(Address::Standard(0x10), &response),
            Err(DdcError::InvalidResponse(
                "VCP response feature code mismatch"
            ))
        );
    }

    #[test]
    fn extended_response_does_not_require_a_standard_feature_echo() {
        let response = [
            0x6E, 0x88, 0x02, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x32, 0xE2,
        ];
        assert!(decode_response(Address::e2a0(0x20), &response).is_ok());
    }

    /// A display's "unsupported" response carries a result code rather than a transport failure.
    #[test]
    fn response_with_result_code_is_reported_as_unsupported() {
        let mut response = [
            0x6E, 0x88, 0x02, 0x01, 0x00, 0x00, 0x00, 0x64, 0x00, 0x32, 0xE2,
        ];
        // Recompute the checksum after changing the result code so the test fails
        // specifically because of that code.
        response[10] = response[..10]
            .iter()
            .fold(0x50, |checksum, byte| checksum ^ byte);
        let error = decode_response(Address::Standard(0x10), &response).unwrap_err();
        assert_eq!(
            error,
            DdcError::Unsupported {
                address: Address::Standard(0x10),
                result_code: 0x01,
            }
        );
    }

    #[test]
    fn capabilities_request_encodes_the_fragment_offset() {
        assert_eq!(
            RequestPacket::capabilities(0x0102).as_slice(),
            &[0x51, 0x83, 0xF3, 0x01, 0x02, 0x4C]
        );
    }

    #[test]
    fn capabilities_response_decodes_its_data_fragment() {
        let mut response = [0_u8; CAPABILITIES_RESPONSE_SIZE];
        response[0] = 0x6E;
        response[1] = 0x87;
        response[2] = 0xE3;
        response[3..5].copy_from_slice(&0x0102_u16.to_be_bytes());
        response[5..9].copy_from_slice(&[0, 2, 4, 138]);
        response[9] = response[..9]
            .iter()
            .fold(0x50, |checksum, byte| checksum ^ byte);
        assert_eq!(
            decode_capabilities_response(0x0102, &response),
            Ok(CapabilityFragment {
                offset: 0x0102,
                data: vec![0, 2, 4, 138],
            })
        );
    }

    #[test]
    fn capabilities_response_rejects_checksum_and_offset_mismatches() {
        let mut response = [0_u8; CAPABILITIES_RESPONSE_SIZE];
        response[0] = 0x6E;
        response[1] = 0x84;
        response[2] = 0xE3;
        response[3..5].copy_from_slice(&32_u16.to_be_bytes());
        response[5] = b'x';
        response[6] = response[..6]
            .iter()
            .fold(0x50, |checksum, byte| checksum ^ byte);
        assert_eq!(
            decode_capabilities_response(0, &response),
            Err(DdcError::InvalidResponse(
                "capabilities fragment offset mismatch"
            ))
        );
        response[6] ^= 0xFF;
        assert_eq!(
            decode_capabilities_response(32, &response),
            Err(DdcError::ChecksumMismatch)
        );
    }
}
