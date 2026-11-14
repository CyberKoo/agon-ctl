use crate::protocol::{
    CAPABILITIES_RESPONSE_SIZE, CapabilityFragment, RequestPacket, VCP_RESPONSE_SIZE,
    decode_capabilities_response, decode_response,
};
use crate::{Address, DdcError, VcpValue};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransportError {
    InvalidRequest(&'static str),
    UnexpectedEof(&'static str),
    Io {
        operation: &'static str,
        code: i32,
    },
    /// The channel does not carry this feature (for example, dxva2 supports only standard VCP).
    /// Treat this as an unsupported capability: skip retries and exclude it from failure counts.
    UnsupportedFeature(&'static str),
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => formatter.write_str(message),
            Self::UnexpectedEof(operation) => {
                write!(formatter, "{operation} completed with a short transfer")
            }
            Self::Io { operation, code } => {
                write!(formatter, "{operation} failed with OS error {code}")
            }
            Self::UnsupportedFeature(message) => formatter.write_str(message),
        }
    }
}

impl Error for TransportError {}

/// Byte-oriented I/O implemented by a single physical channel.
pub trait PacketTransport {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError>;
    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DdcRequest {
    Vcp(Address),
    Capabilities,
}

impl DdcRequest {
    pub fn unsupported_error(self) -> DdcError {
        match self {
            Self::Vcp(address) => DdcError::UnsupportedFeature { address },
            Self::Capabilities => DdcError::UnsupportedCapabilities,
        }
    }
}

/// DDC/CI operations. Implementations must not retry internally:
/// the client owns the three-attempt budget.
pub trait DdcTransport {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError>;
    fn write_vcp(&mut self, address: Address, value: u16) -> Result<(), DdcError>;

    fn read_capabilities(&mut self, offset: u16) -> Result<CapabilityFragment, DdcError> {
        let _ = offset;
        Err(DdcError::UnsupportedCapabilities)
    }

    /// Prepares a subsequent read attempt without doing I/O. Multi-channel transports
    /// may select another eligible channel here, never midway through a transaction.
    fn recover_read(&mut self, _address: Address, error: &DdcError) -> bool {
        !matches!(
            error,
            DdcError::Unsupported { .. }
                | DdcError::UnsupportedFeature { .. }
                | DdcError::Transport(TransportError::InvalidRequest(_))
        )
    }

    fn recover_capabilities(&mut self, error: &DdcError) -> bool {
        !matches!(
            error,
            DdcError::Unsupported { .. }
                | DdcError::UnsupportedCapabilities
                | DdcError::Transport(TransportError::InvalidRequest(_))
        )
    }
}

impl<T: PacketTransport> DdcTransport for T {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        let packet = RequestPacket::read(address);
        self.write_packet(packet.as_slice())
            .map_err(|error| classify(error, DdcRequest::Vcp(address)))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut response = [0; VCP_RESPONSE_SIZE];
        self.read_packet(&mut response)
            .map_err(|error| classify(error, DdcRequest::Vcp(address)))?;
        decode_response(address, &response)
    }

    fn write_vcp(&mut self, address: Address, value: u16) -> Result<(), DdcError> {
        let packet = RequestPacket::write(address, value);
        self.write_packet(packet.as_slice())
            .map_err(|error| classify(error, DdcRequest::Vcp(address)))
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<CapabilityFragment, DdcError> {
        let packet = RequestPacket::capabilities(offset);
        self.write_packet(packet.as_slice())
            .map_err(|error| classify(error, DdcRequest::Capabilities))?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut response = [0; CAPABILITIES_RESPONSE_SIZE];
        self.read_packet(&mut response)
            .map_err(|error| classify(error, DdcRequest::Capabilities))?;
        decode_capabilities_response(offset, &response)
    }
}

fn classify(error: TransportError, request: DdcRequest) -> DdcError {
    match error {
        TransportError::UnsupportedFeature(_) => request.unsupported_error(),
        other => DdcError::from(other),
    }
}
