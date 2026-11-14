use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum WindowsError {
    Library {
        provider: &'static str,
        source: io::Error,
    },
    MissingSymbol {
        provider: &'static str,
        symbol: &'static str,
    },
    Call {
        provider: &'static str,
        operation: &'static str,
        code: i32,
    },
    InvalidData(&'static str),
    NoRawChannel,
}

impl fmt::Display for WindowsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library { provider, source } => {
                write!(formatter, "failed to load {provider}: {source}")
            }
            Self::MissingSymbol { provider, symbol } => {
                write!(formatter, "{provider} does not export {symbol}")
            }
            Self::Call {
                provider,
                operation,
                code,
            } => write!(
                formatter,
                "{provider} {operation} failed with status {code}"
            ),
            Self::InvalidData(message) => formatter.write_str(message),
            Self::NoRawChannel => formatter.write_str("no raw DDC/CI channel is available"),
        }
    }
}

impl Error for WindowsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library { source, .. } => Some(source),
            _ => None,
        }
    }
}
