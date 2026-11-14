#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    pub fn CGGetOnlineDisplayList(
        max_displays: u32,
        online_displays: *mut u32,
        display_count: *mut u32,
    ) -> i32;
    pub fn CGDisplayIsBuiltin(display: u32) -> i32;
    pub fn CGDisplayVendorNumber(display: u32) -> u32;
    pub fn CGDisplayModelNumber(display: u32) -> u32;
    pub fn CGDisplaySerialNumber(display: u32) -> u32;
}
