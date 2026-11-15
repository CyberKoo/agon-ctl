use crate::identity::merge_groups;
use crate::{WindowsDisplay, WindowsError, dxva2};

/// Enumerates displays without emitting recoverable diagnostics.
pub fn enumerate() -> Result<Vec<WindowsDisplay>, WindowsError> {
    enumerate_with_diagnostics(|_| {})
}

/// Enumerates displays and reports recoverable diagnostics to the caller.
pub fn enumerate_with_diagnostics(
    mut diagnostic: impl FnMut(String),
) -> Result<Vec<WindowsDisplay>, WindowsError> {
    let mut displays = Vec::new();
    let mut deferred_diagnostics = Vec::new();
    let mut first_error = None;
    match dxva2::enumerate() {
        Ok(standard) => displays.extend(standard.into_iter().map(WindowsDisplay::from_standard)),
        Err(error) => {
            deferred_diagnostics.push(format!("dxva2 enumeration failed: {error}"));
            first_error = Some(error);
        }
    }

    for (provider, result) in [
        (
            "IGCL",
            crate::sdk::igcl::IgclProvider::new()
                .and_then(|provider| provider.enumerate(&mut diagnostic)),
        ),
        (
            "ADL",
            crate::sdk::adl::AdlProvider::new()
                .and_then(|provider| provider.enumerate(&mut diagnostic)),
        ),
        (
            "NvAPI",
            crate::sdk::nvapi::NvProvider::new()
                .and_then(|provider| provider.enumerate(&mut diagnostic)),
        ),
    ] {
        match result {
            Ok(raw) => displays.extend(raw.into_iter().map(WindowsDisplay::from_raw)),
            Err(error) => {
                deferred_diagnostics.push(format!("{provider} enumeration failed: {error}"));
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if displays.is_empty() {
        for warning in deferred_diagnostics {
            diagnostic(warning);
        }
        if let Some(error) = first_error {
            return Err(error);
        }
    }

    // Plan against ALL original outputs, never against an incrementally merged list.
    let identities: Vec<_> = displays.iter().map(WindowsDisplay::identity).collect();
    let groups = merge_groups(&identities, &mut diagnostic);
    let mut merged: Vec<WindowsDisplay> = Vec::new();
    let mut destinations: Vec<Option<usize>> = vec![None; displays.len()];
    for (index, display) in displays.into_iter().enumerate() {
        let group = groups[index];
        if let Some(destination) = destinations[group] {
            merged[destination].merge(display);
        } else {
            destinations[group] = Some(merged.len());
            merged.push(display);
        }
    }
    Ok(merged)
}
