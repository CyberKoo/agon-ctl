use crate::{edid, i2c::I2cDevice};
use agon_osd::{Address, DdcError, DdcTransport, EdidIdentity, TransportError};
use std::io;
use std::path::Path;

const DDC_CI_ADDRESS: u16 = 0x37;

pub(crate) struct ProbedDevice<T> {
    pub(crate) device: T,
    pub(crate) identity: Option<EdidIdentity>,
}

#[derive(Debug)]
pub(crate) enum ProbeError {
    NoDisplay,
    InvalidResponse(DdcError),
    PermissionDenied,
}

pub(crate) fn probe_device(path: &Path) -> Result<ProbedDevice<I2cDevice>, ProbeError> {
    let mut device = I2cDevice::open(path, DDC_CI_ADDRESS).map_err(probe_io_error)?;
    probe_transport(&mut device)?;
    let identity = edid::read_identity(path);
    Ok(ProbedDevice { device, identity })
}

fn probe_transport(transport: &mut impl DdcTransport) -> Result<(), ProbeError> {
    match transport.read_vcp(Address::Standard(0x10)) {
        Ok(_) | Err(DdcError::Unsupported { .. }) => Ok(()),
        Err(error @ (DdcError::InvalidResponse(_) | DdcError::ChecksumMismatch)) => {
            Err(ProbeError::InvalidResponse(error))
        }
        Err(DdcError::Transport(TransportError::Io { code: 1 | 13, .. })) => {
            Err(ProbeError::PermissionDenied)
        }
        Err(_) => Err(ProbeError::NoDisplay),
    }
}

fn probe_io_error(error: io::Error) -> ProbeError {
    if error.kind() == io::ErrorKind::PermissionDenied
        || matches!(error.raw_os_error(), Some(1 | 13))
    {
        ProbeError::PermissionDenied
    } else {
        ProbeError::NoDisplay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agon_osd::PacketTransport;

    struct MockTransport {
        response: [u8; 11],
        writes: Vec<Vec<u8>>,
    }

    impl PacketTransport for MockTransport {
        fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
            self.writes.push(packet.to_vec());
            Ok(())
        }

        fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
            response.copy_from_slice(&self.response);
            Ok(())
        }
    }

    fn mock_transport(result_code: u8) -> MockTransport {
        let mut response = [0; 11];
        response[0] = 0x6E;
        response[1] = 0x88;
        response[2] = 0x02;
        response[3] = result_code;
        response[4] = 0x10;
        response[7] = 100;
        response[9] = 50;
        response[10] = response[..10]
            .iter()
            .fold(0x50, |checksum, byte| checksum ^ byte);
        MockTransport {
            response,
            writes: Vec::new(),
        }
    }

    #[test]
    fn aoc_probe_sends_one_brightness_read_and_validates_the_response() {
        for result_code in [0, 1] {
            let mut transport = mock_transport(result_code);
            assert!(probe_transport(&mut transport).is_ok());
            assert_eq!(transport.writes, [[0x51, 0x82, 0x01, 0x10, 0xAC]]);
        }

        let mut transport = mock_transport(0);
        transport.response[0] = 0;
        assert!(matches!(
            probe_transport(&mut transport),
            Err(ProbeError::InvalidResponse(_))
        ));
        assert_eq!(transport.writes.len(), 1);
    }
}
