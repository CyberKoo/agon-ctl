#[cfg(target_os = "linux")]
pub(crate) use agon_osd_linux::{
    LinuxDisplay as NativeDisplay, LinuxDisplayError as PlatformError,
};
#[cfg(target_os = "macos")]
pub(crate) use agon_osd_macos::{MacDisplay as NativeDisplay, MacDisplayError as PlatformError};
#[cfg(target_os = "windows")]
pub(crate) use agon_osd_windows::{WindowsDisplay as NativeDisplay, WindowsError as PlatformError};

pub(crate) fn enumerate() -> Result<Vec<NativeDisplay>, PlatformError> {
    #[cfg(target_os = "linux")]
    {
        agon_osd_linux::enumerate_with_diagnostics(|warning| eprintln!("warning: {warning}"))
    }
    #[cfg(target_os = "macos")]
    {
        agon_osd_macos::enumerate()
    }
    #[cfg(target_os = "windows")]
    {
        agon_osd_windows::enumerate_with_diagnostics(|warning| eprintln!("warning: {warning}"))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("agon-ctl currently supports macOS, Linux and Windows");
