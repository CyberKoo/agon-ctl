use agon_osd_macos_sys::core_graphics as bindings;

pub(crate) fn online_external_display_ids() -> Result<Vec<u32>, i32> {
    let mut count = 0_u32;
    // SAFETY: a null display buffer is valid when `max_displays` is zero.
    let status = unsafe { bindings::CGGetOnlineDisplayList(0, std::ptr::null_mut(), &mut count) };
    if status != 0 {
        return Err(status);
    }
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut displays = vec![0; count as usize];
    // SAFETY: `displays` has capacity for `count` IDs and `count` remains writable.
    let status =
        unsafe { bindings::CGGetOnlineDisplayList(count, displays.as_mut_ptr(), &mut count) };
    if status != 0 {
        return Err(status);
    }
    displays.truncate(count as usize);
    displays.retain(|display| {
        // SAFETY: IDs returned by CGGetOnlineDisplayList are valid for this query.
        let is_builtin = unsafe { bindings::CGDisplayIsBuiltin(*display) };
        is_builtin == 0
    });
    Ok(displays)
}

pub(crate) fn vendor_number(display: u32) -> u32 {
    // SAFETY: the display ID came from the current online display list.
    unsafe { bindings::CGDisplayVendorNumber(display) }
}

pub(crate) fn model_number(display: u32) -> u32 {
    // SAFETY: the display ID came from the current online display list.
    unsafe { bindings::CGDisplayModelNumber(display) }
}

pub(crate) fn serial_number(display: u32) -> u32 {
    // SAFETY: the display ID came from the current online display list.
    unsafe { bindings::CGDisplaySerialNumber(display) }
}
