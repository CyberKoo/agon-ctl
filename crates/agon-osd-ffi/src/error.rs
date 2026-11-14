use agon_osd::{DdcError, OsdError};

pub(crate) const OK: u32 = 0;
pub(crate) const INVALID_ARGUMENT: u32 = 1;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub(crate) const UNSUPPORTED_PLATFORM: u32 = 2;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", test))]
pub(crate) const DISPLAY_NOT_FOUND: u32 = 3;
pub(crate) const UNSUPPORTED_FEATURE: u32 = 4;
pub(crate) const VALUE_ABOVE_MAXIMUM: u32 = 5;
pub(crate) const TRANSPORT_ERROR: u32 = 6;
pub(crate) const INVALID_RESPONSE: u32 = 7;
pub(crate) const WRITE_UNCERTAIN: u32 = 8;
pub(crate) const READBACK_FAILED: u32 = 9;
pub(crate) const INTERNAL_ERROR: u32 = 10;

#[derive(Debug)]
pub(crate) struct ApiError {
    pub(crate) status: u32,
    pub(crate) message: String,
}

impl ApiError {
    pub(crate) fn new(status: u32, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(INTERNAL_ERROR, message)
    }
}

impl From<OsdError> for ApiError {
    fn from(error: OsdError) -> Self {
        let status = match &error {
            OsdError::WriteUncertain(_) => WRITE_UNCERTAIN,
            OsdError::ReadbackFailed(_) => READBACK_FAILED,
            OsdError::InvalidValue(_) => INVALID_ARGUMENT,
            OsdError::InvalidCapabilities(_) => INVALID_RESPONSE,
            OsdError::ValueAboveMaximum { .. } => VALUE_ABOVE_MAXIMUM,
            OsdError::Ddc(
                DdcError::Unsupported { .. }
                | DdcError::UnsupportedFeature { .. }
                | DdcError::UnsupportedCapabilities,
            ) => UNSUPPORTED_FEATURE,
            OsdError::Ddc(DdcError::Transport(_)) => TRANSPORT_ERROR,
            OsdError::Ddc(DdcError::InvalidResponse(_) | DdcError::ChecksumMismatch) => {
                INVALID_RESPONSE
            }
        };
        Self::new(status, error.to_string())
    }
}
