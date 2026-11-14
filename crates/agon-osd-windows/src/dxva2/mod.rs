mod discovery;
mod registry_edid;
mod transport;

pub(crate) use discovery::enumerate;
pub(crate) use transport::is_refreshed_monitor_error;

use crate::WindowsError;
use agon_osd::{DisplayInfo, TransportError};
use windows_sys::Win32::Devices::Display::{DestroyPhysicalMonitors, PHYSICAL_MONITOR};
use windows_sys::Win32::Foundation::ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS;

pub(crate) struct Dxva2Display {
    info: DisplayInfo,
    edid: Option<[u8; 128]>,
    monitor: PHYSICAL_MONITOR,
    pending_read: Option<u8>,
}

impl Dxva2Display {
    pub(crate) fn info(&self) -> &DisplayInfo {
        &self.info
    }

    pub(crate) fn edid(&self) -> Option<[u8; 128]> {
        self.edid
    }

    fn refresh_monitor(&mut self) -> Result<(), TransportError> {
        let id = self.info.id().to_owned();
        let description = self.monitor.szPhysicalMonitorDescription;
        let description = wide_to_string(&description);
        let mut matches = enumerate()
            .map_err(refresh_error)?
            .into_iter()
            .filter(|display| same_physical_display(&id, &description, display));
        let mut replacement = matches.next().ok_or_else(no_unique_monitor_error)?;
        if matches.next().is_some() {
            return Err(no_unique_monitor_error());
        }

        std::mem::swap(&mut self.monitor, &mut replacement.monitor);
        // The replacement now owns the old handle and destroys it on drop.
        self.pending_read = None;
        Ok(())
    }
}

fn refresh_error(error: WindowsError) -> TransportError {
    let code = match error {
        WindowsError::Call { code, .. } => code,
        WindowsError::Library { source, .. } => source.raw_os_error().unwrap_or(-1),
        _ => -1,
    };
    TransportError::Io {
        operation: "dxva2 physical-monitor reenumeration",
        code,
    }
}

fn no_unique_monitor_error() -> TransportError {
    TransportError::Io {
        operation: "dxva2 physical-monitor reenumeration found no unique match",
        code: ERROR_GRAPHICS_MONITOR_NO_LONGER_EXISTS,
    }
}

fn same_physical_display(
    current_id: &str,
    current_description: &str,
    candidate: &Dxva2Display,
) -> bool {
    let description = candidate.monitor.szPhysicalMonitorDescription;
    let candidate_description = wide_to_string(&description);
    same_display_identity(
        current_id,
        current_description,
        candidate.info.id(),
        &candidate_description,
    )
}

fn same_display_identity(
    current_id: &str,
    current_description: &str,
    candidate_id: &str,
    candidate_description: &str,
) -> bool {
    if current_id.starts_with("dxva2:") && !current_id.starts_with("dxva2:fallback:") {
        current_id.eq_ignore_ascii_case(candidate_id)
    } else {
        !current_description.is_empty() && current_description == candidate_description
    }
}

impl Drop for Dxva2Display {
    fn drop(&mut self) {
        // SAFETY: the physical monitor handle is owned by this display and destroyed once.
        unsafe { DestroyPhysicalMonitors(1, &self.monitor) };
    }
}

fn wide_to_string(value: &[u16]) -> String {
    let end = value
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..end]).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_device_path_is_case_insensitive_and_ignores_description_changes() {
        assert!(same_display_identity(
            "dxva2:Display#AOC#ABC",
            "old description",
            "DXVA2:display#aoc#abc",
            "new description"
        ));
        assert!(!same_display_identity(
            "dxva2:Display#AOC#ABC",
            "same description",
            "dxva2:Display#AOC#DEF",
            "same description"
        ));
    }

    #[test]
    fn fallback_identity_requires_a_nonempty_matching_description() {
        let current = "dxva2:fallback:process-1:token-0";
        let replacement = "dxva2:fallback:process-1:token-1";
        assert!(same_display_identity(current, "AGON", replacement, "AGON"));
        assert!(!same_display_identity(
            current,
            "AGON",
            replacement,
            "OTHER"
        ));
        assert!(!same_display_identity(current, "", replacement, ""));
    }
}
