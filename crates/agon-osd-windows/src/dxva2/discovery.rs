use super::registry_edid::read_edid_from_registry;
use super::{Dxva2Display, wide_to_string};
use crate::{WindowsError, identity};
use agon_osd::{DisplayInfo, parse_edid};
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Devices::Display::{
    GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
};
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    DISPLAY_DEVICE_ACTIVE, DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors,
    GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::EDD_GET_DEVICE_INTERFACE_NAME;

pub(crate) fn enumerate() -> Result<Vec<Dxva2Display>, WindowsError> {
    let mut monitors = Vec::new();
    // SAFETY: the callback receives a valid vector pointer for the duration of the synchronous enumeration.
    let result = unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(collect_monitor),
            (&mut monitors as *mut Vec<HMONITOR>) as isize,
        )
    };
    if result == 0 {
        return Err(last_windows_error("EnumDisplayMonitors"));
    }

    let mut displays = Vec::new();
    for monitor in monitors {
        let mut count = 0;
        // SAFETY: the HMONITOR comes from EnumDisplayMonitors and the count pointer is valid.
        let result = unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) };
        if result == 0 || count == 0 {
            continue;
        }
        let mut physical = vec![PHYSICAL_MONITOR::default(); count as usize];
        // SAFETY: `physical` has capacity for `count` structures.
        let result =
            unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, count, physical.as_mut_ptr()) };
        if result == 0 {
            continue;
        }
        let device_path = monitor_device_path(monitor, count);
        // A physical handle cannot be matched to a particular interface when
        // either side has multiple entries. Never distribute one EDID to all.
        let edid = device_path.as_deref().and_then(read_edid_from_registry);
        for monitor in physical {
            let description = monitor.szPhysicalMonitorDescription;
            let name = wide_to_string(&description);
            let identity = edid.as_ref().and_then(|bytes| parse_edid(bytes));
            let location = device_path
                .as_ref()
                .map(|path| format!("dxva2:{path}"))
                .unwrap_or_else(fallback_location);
            let info = DisplayInfo::new(
                location.clone(),
                identity
                    .as_ref()
                    .and_then(|identity| identity.name.clone())
                    .or_else(|| (!name.is_empty()).then_some(name))
                    .unwrap_or_else(|| location.clone()),
                identity.as_ref().map_or(0, |identity| identity.vendor_id),
                identity.as_ref().map_or(0, |identity| identity.model_id),
                identity
                    .as_ref()
                    .map_or(0, |identity| identity.serial_number),
                location,
            );
            displays.push(Dxva2Display {
                info,
                edid,
                monitor,
                pending_read: None,
            });
        }
    }
    Ok(displays)
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: isize,
) -> i32 {
    // SAFETY: `data` points to Vec<HMONITOR> for the duration of EnumDisplayMonitors.
    let monitors = unsafe { &mut *(data as *mut Vec<HMONITOR>) };
    monitors.push(monitor);
    1
}

fn last_windows_error(operation: &'static str) -> WindowsError {
    WindowsError::Call {
        provider: "dxva2",
        operation,
        code: std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
    }
}

/// Returns an opaque monitor interface path, not an adapter DeviceID or a name.
fn monitor_device_path(monitor: HMONITOR, physical_count: u32) -> Option<String> {
    if physical_count != 1 {
        return None;
    }
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: MONITORINFOEXW starts with MONITORINFO; cbSize describes the full buffer.
    if unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) } == 0 {
        return None;
    }
    if info.szDevice[0] == 0 {
        return None;
    }
    let mut paths = Vec::new();
    for index in 0.. {
        let mut device = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        // SAFETY: szDevice is the NUL-terminated GDI adapter name returned by
        // GetMonitorInfoW. This enumerates its monitors, not top-level adapters.
        let result = unsafe {
            EnumDisplayDevicesW(
                info.szDevice.as_ptr(),
                index,
                &mut device,
                EDD_GET_DEVICE_INTERFACE_NAME,
            )
        };
        if result == 0 {
            break;
        }
        if device.StateFlags & DISPLAY_DEVICE_ACTIVE != 0 {
            paths.push(wide_to_string(&device.DeviceID));
        }
    }
    identity::unique_monitor_path(physical_count, &paths)
}

/// Explicitly ephemeral: not suitable for persistence or cross-backend matching.
/// No Win32 handle or enumeration order is presented as a stable device identity.
fn fallback_location() -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    format!(
        "dxva2:fallback:process-{}:token-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}
