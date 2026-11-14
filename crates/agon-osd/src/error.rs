use crate::{Address, CapabilityParseError, TransportError};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DdcError {
    Transport(TransportError),
    InvalidResponse(&'static str),
    ChecksumMismatch,
    Unsupported {
        address: Address,
        result_code: u8,
    },
    /// The channel lacks this feature (for example, dxva2 supports only standard VCP); this is not a failure.
    UnsupportedFeature {
        address: Address,
    },
    UnsupportedCapabilities,
}

impl fmt::Display for DdcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => error.fmt(formatter),
            Self::InvalidResponse(message) => formatter.write_str(message),
            Self::ChecksumMismatch => formatter.write_str("DDC response checksum mismatch"),
            Self::Unsupported {
                address,
                result_code,
            } => write!(
                formatter,
                "display rejected address {address} with result code {result_code}"
            ),
            Self::UnsupportedFeature { address } => {
                write!(formatter, "channel does not support address {address}")
            }
            Self::UnsupportedCapabilities => {
                formatter.write_str("channel does not support capabilities requests")
            }
        }
    }
}

impl Error for DdcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transport(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TransportError> for DdcError {
    fn from(error: TransportError) -> Self {
        Self::Transport(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OsdError {
    Ddc(DdcError),
    InvalidValue(crate::ValueParseError),
    InvalidCapabilities(CapabilityParseError),
    /// The transport failed during a write; the display may have applied it.
    WriteUncertain(DdcError),
    /// A write completed, but its result could not be read back.
    ReadbackFailed(DdcError),
    ValueAboveMaximum {
        value: u16,
        maximum: u16,
    },
}

impl fmt::Display for OsdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ddc(error) => error.fmt(formatter),
            Self::InvalidValue(error) => error.fmt(formatter),
            Self::InvalidCapabilities(error) => error.fmt(formatter),
            Self::WriteUncertain(error) => write!(
                formatter,
                "write outcome is uncertain; command was not retried; read the current value before retrying: {error}"
            ),
            Self::ReadbackFailed(error) => write!(
                formatter,
                "write completed but readback failed; the setting may already have changed: {error}"
            ),
            Self::ValueAboveMaximum { value, maximum } => {
                write!(formatter, "value {value} exceeds display maximum {maximum}")
            }
        }
    }
}

impl Error for OsdError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ddc(error) | Self::WriteUncertain(error) | Self::ReadbackFailed(error) => {
                Some(error)
            }
            Self::InvalidValue(error) => Some(error),
            Self::InvalidCapabilities(error) => Some(error),
            Self::ValueAboveMaximum { .. } => None,
        }
    }
}

impl From<DdcError> for OsdError {
    fn from(error: DdcError) -> Self {
        Self::Ddc(error)
    }
}

impl From<crate::ValueParseError> for OsdError {
    fn from(error: crate::ValueParseError) -> Self {
        Self::InvalidValue(error)
    }
}
