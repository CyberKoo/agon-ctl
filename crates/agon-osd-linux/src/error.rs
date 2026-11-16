use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum LinuxDisplayError {
    Io { path: PathBuf, source: io::Error },
}

impl fmt::Display for LinuxDisplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
        }
    }
}

impl Error for LinuxDisplayError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
        }
    }
}
