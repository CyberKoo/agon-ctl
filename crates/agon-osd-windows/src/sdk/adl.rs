use super::{DiscoveredDisplay, RawDisplay};
use crate::{Library, WindowsError};
use agon_osd::{DisplayInfo, PacketTransport, TransportError, parse_edid};
use agon_osd_windows_sys::adl::*;
use std::collections::HashSet;
use std::ffi::c_void;
use std::rc::Rc;

const PROVIDER: &str = "AMD ADL";

pub(crate) struct AdlProvider {
    _library: Rc<Library>,
    context: AdlContextHandle,
    destroy: Adl2MainControlDestroy,
    adapter_count: Adl2AdapterNumberOfAdaptersGet,
    display_info: Adl2DisplayDisplayInfoGet,
    get_edid: Adl2DisplayEdidDataGet,
    ddc_access: Adl2DisplayDdcBlockAccessGet,
}

impl AdlProvider {
    pub(crate) fn new() -> Result<Rc<Self>, WindowsError> {
        let library = Rc::new(Library::load(PROVIDER, &["atiadlxx.dll", "atiadlxy.dll"])?);
        let create: Adl2MainControlCreate = library.symbol(c"ADL2_Main_Control_Create")?;
        let destroy = library.symbol(c"ADL2_Main_Control_Destroy")?;
        let adapter_count = library.symbol(c"ADL2_Adapter_NumberOfAdapters_Get")?;
        let display_info = library.symbol(c"ADL2_Display_DisplayInfo_Get")?;
        let get_edid = library.symbol(c"ADL2_Display_EdidData_Get")?;
        let ddc_access = library.symbol(c"ADL2_Display_DDCBlockAccess_Get")?;
        let mut context = std::ptr::null_mut();
        // SAFETY: the callback matches ADL_MAIN_MALLOC_CALLBACK and the context pointer is valid.
        let status = unsafe { create(malloc_callback, 1, &mut context) };
        check(status, "ADL2_Main_Control_Create")?;
        if context.is_null() {
            return Err(WindowsError::InvalidData("ADL returned a null context"));
        }
        Ok(Rc::new(Self {
            _library: library,
            context,
            destroy,
            adapter_count,
            display_info,
            get_edid,
            ddc_access,
        }))
    }

    pub(crate) fn enumerate(
        self: &Rc<Self>,
        diagnostic: &mut dyn FnMut(String),
    ) -> Result<Vec<DiscoveredDisplay>, WindowsError> {
        let mut count = 0;
        // SAFETY: the ADL context is valid and the count pointer is writable.
        let status = unsafe { (self.adapter_count)(self.context, &mut count) };
        check(status, "ADL2_Adapter_NumberOfAdapters_Get")?;
        let mut displays = Vec::new();
        let mut seen_outputs = HashSet::new();
        for adapter_index in 0..count {
            let mut display_count = 0;
            let mut infos = std::ptr::null_mut();
            // SAFETY: the output pointers are valid; ADL allocates the display array through our callback.
            let status = unsafe {
                (self.display_info)(
                    self.context,
                    adapter_index,
                    &mut display_count,
                    &mut infos,
                    0,
                )
            };
            if let Err(error) = check(status, "ADL2_Display_DisplayInfo_Get") {
                diagnostic(format!("{PROVIDER} adapter {adapter_index}: {error}"));
                // SAFETY: any non-null allocation was made through our CRT callback,
                // including one returned alongside a failed call.
                unsafe { libc::free(infos.cast()) };
                continue;
            }
            if infos.is_null() {
                if display_count > 0 {
                    diagnostic(format!(
                        "{PROVIDER} adapter {adapter_index}: null display list"
                    ));
                }
                continue;
            }
            if display_count <= 0 {
                // SAFETY: ADL allocated the array through the matching allocation callback.
                unsafe { libc::free(infos.cast()) };
                continue;
            }
            // SAFETY: ADL returned an array containing `display_count` entries.
            let infos_slice = unsafe { std::slice::from_raw_parts(infos, display_count as usize) };
            for info in infos_slice {
                if !is_connected(info) {
                    continue;
                }
                // ADL EDID/DDC calls take the logical display index, not the
                // physical index or either adapter field in ADLDisplayID.
                let display_index = info.display_id.display_logical_index;
                let mut edid = AdlDisplayEdidData::default();
                // SAFETY: the adapter/display IDs and EDID structure are valid.
                let status = unsafe {
                    (self.get_edid)(self.context, adapter_index, display_index, &mut edid)
                };
                if let Err(error) = check(status, "ADL2_Display_EdidData_Get") {
                    diagnostic(format!(
                        "{PROVIDER} adapter {adapter_index} output {display_index}: {error}"
                    ));
                    continue;
                }
                let Some(size) = usize::try_from(edid.edid_size)
                    .ok()
                    .filter(|size| (128..=256).contains(size))
                else {
                    diagnostic(format!(
                        "{PROVIDER} adapter {adapter_index} output {display_index}: invalid EDID size {}",
                        edid.edid_size
                    ));
                    continue;
                };
                // SAFETY: the validated size is within the initialized EDID array.
                let bytes = unsafe {
                    std::slice::from_raw_parts(edid.edid_data.as_ptr().cast::<u8>(), size)
                };
                let Some(identity) = parse_edid(bytes) else {
                    diagnostic(format!(
                        "{PROVIDER} adapter {adapter_index} output {display_index}: invalid EDID base block"
                    ));
                    continue;
                };
                // Logical adapters can expose aliases of one physical output. Only
                // deduplicate that explicit ADL identity, never equal EDID alone.
                if physical_output(info).is_some_and(|output| !seen_outputs.insert(output)) {
                    continue;
                }
                let mut key = [0; 128];
                key.copy_from_slice(&bytes[..128]);
                let location = format!("adl:{adapter_index}:{display_index}");
                let name = identity.name.unwrap_or_else(|| display_name(info));
                let info_out = DisplayInfo::new(
                    location.clone(),
                    name,
                    identity.vendor_id,
                    identity.model_id,
                    identity.serial_number,
                    location,
                );
                displays.push(DiscoveredDisplay {
                    info: info_out,
                    edid: key,
                    raw: RawDisplay::Amd(AdlDisplay {
                        provider: Rc::clone(self),
                        adapter_index,
                        display_index,
                    }),
                });
            }
            // SAFETY: ADL allocated this array through the matching CRT allocation callback.
            unsafe { libc::free(infos.cast()) };
        }
        Ok(displays)
    }
}

impl Drop for AdlProvider {
    fn drop(&mut self) {
        // SAFETY: the context remains valid until all dependent displays release the provider.
        unsafe { (self.destroy)(self.context) };
    }
}

pub(crate) struct AdlDisplay {
    provider: Rc<AdlProvider>,
    adapter_index: i32,
    display_index: i32,
}

impl PacketTransport for AdlDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        let mut message = Vec::with_capacity(packet.len() + 1);
        message.push(0x6E);
        message.extend_from_slice(packet);
        let mut received = 0;
        // SAFETY: the adapter/display IDs and message buffer are valid for this synchronous call.
        let status = unsafe {
            (self.provider.ddc_access)(
                self.provider.context,
                self.adapter_index,
                self.display_index,
                0,
                0,
                message.len() as i32,
                message.as_mut_ptr().cast(),
                &mut received,
                std::ptr::null_mut(),
            )
        };
        if status == ADL_OK {
            Ok(())
        } else {
            Err(TransportError::Io {
                operation: "ADL2_Display_DDCBlockAccess_Get(write)",
                code: status,
            })
        }
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        let mut command = [0x6F];
        let mut received = response.len() as i32;
        // SAFETY: the adapter/display IDs and command/response buffers are valid for this call.
        let status = unsafe {
            (self.provider.ddc_access)(
                self.provider.context,
                self.adapter_index,
                self.display_index,
                0,
                0,
                command.len() as i32,
                command.as_mut_ptr().cast(),
                &mut received,
                response.as_mut_ptr().cast(),
            )
        };
        if status != ADL_OK {
            return Err(TransportError::Io {
                operation: "ADL2_Display_DDCBlockAccess_Get(read)",
                code: status,
            });
        }
        if usize::try_from(received).ok() != Some(response.len()) {
            return Err(TransportError::UnexpectedEof(
                "ADL2_Display_DDCBlockAccess_Get(read)",
            ));
        }
        Ok(())
    }
}

unsafe extern "system" fn malloc_callback(size: i32) -> *mut c_void {
    // SAFETY: a positive ADL allocation size is forwarded to the process CRT allocator.
    unsafe { libc::malloc(size as usize) }
}

fn check(status: i32, operation: &'static str) -> Result<(), WindowsError> {
    if status == ADL_OK {
        Ok(())
    } else {
        Err(WindowsError::Call {
            provider: PROVIDER,
            operation,
            code: status,
        })
    }
}

fn is_connected(info: &AdlDisplayInfo) -> bool {
    let flag = ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED;
    info.display_info_mask & flag != 0 && info.display_info_value & flag != 0
}

fn physical_output(info: &AdlDisplayInfo) -> Option<(i32, i32)> {
    let id = info.display_id;
    (id.display_physical_adapter_index >= 0 && id.display_physical_index >= 0)
        .then_some((id.display_physical_adapter_index, id.display_physical_index))
}

fn display_name(info: &AdlDisplayInfo) -> String {
    let length = info
        .display_name
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(info.display_name.len());
    let bytes = info.display_name[..length]
        .iter()
        .map(|character| *character as u8)
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&bytes).trim().to_owned()
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::super::tests::{edid, library};
    use super::*;

    unsafe extern "system" fn destroy(_: AdlContextHandle) -> i32 {
        ADL_OK
    }
    unsafe extern "system" fn adapters(_: AdlContextHandle, count: *mut i32) -> i32 {
        // SAFETY: the caller supplies a writable count.
        unsafe { *count = 4 };
        ADL_OK
    }

    unsafe extern "system" fn infos(
        _: AdlContextHandle,
        adapter: i32,
        count: *mut i32,
        infos: *mut *mut AdlDisplayInfo,
        _: i32,
    ) -> i32 {
        let indices: &[i32] = if adapter == 0 {
            &[11, 12, 13, 14, 15, 17, 20, 21]
        } else {
            &[16]
        };
        // SAFETY: allocate using the same CRT as the ADL callback and initialize each entry.
        unsafe {
            let data = libc::malloc(indices.len() * std::mem::size_of::<AdlDisplayInfo>())
                .cast::<AdlDisplayInfo>();
            if data.is_null() {
                return -9;
            }
            *count = indices.len() as i32;
            *infos = data;
            for (index, logical) in indices.iter().enumerate() {
                data.add(index).write(AdlDisplayInfo {
                    display_id: AdlDisplayId {
                        display_logical_index: *logical,
                        // Output 17 is a failing alias of physical output 16.
                        display_physical_index: if *logical == 17 { 16 } else { *logical },
                        display_logical_adapter_index: 33,
                        display_physical_adapter_index: 44,
                    },
                    // A driver-supplied name must not rescue an invalid EDID identity.
                    display_name: [b'X' as _; 256],
                    display_info_mask: if *logical == 21 {
                        0
                    } else {
                        ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED
                    },
                    display_info_value: if *logical == 20 {
                        0
                    } else {
                        ADL_DISPLAY_DISPLAYINFO_DISPLAYCONNECTED
                    },
                    ..Default::default()
                });
            }
        }
        // Also exercise freeing an allocation returned alongside an adapter error.
        if adapter == 1 { -1 } else { ADL_OK }
    }

    unsafe extern "system" fn get_edid(
        _: AdlContextHandle,
        adapter: i32,
        display: i32,
        data: *mut AdlDisplayEdidData,
    ) -> i32 {
        if !matches!((adapter, display), (0, 11..=17) | (2..=3, 16)) || matches!(display, 12 | 17) {
            return -2;
        }
        // SAFETY: the caller supplies a writable EDID structure.
        unsafe {
            (*data).edid_size = match display {
                13 => -1,
                14 => 64,
                _ => 128,
            };
            for (index, value) in edid().iter().enumerate() {
                (*data).edid_data[index] = *value as _;
            }
        }
        ADL_OK
    }

    unsafe extern "system" fn invalid_edid(
        context: AdlContextHandle,
        adapter: i32,
        display: i32,
        data: *mut AdlDisplayEdidData,
    ) -> i32 {
        // SAFETY: forward the caller's valid EDID output buffer to the base mock.
        let status = unsafe { get_edid(context, adapter, display, data) };
        if status == ADL_OK {
            // SAFETY: the initialized output structure is writable for the whole call.
            unsafe {
                (*data).edid_size = 128;
                match display {
                    13 => (*data).edid_data[0] = 1,    // Invalid header.
                    14 => (*data).edid_data[127] ^= 1, // Valid header, invalid checksum.
                    _ => {}
                }
            }
        }
        status
    }

    #[test]
    fn invalid_edid_with_driver_name_is_skipped_without_losing_valid_outputs() {
        let provider = Rc::new(AdlProvider {
            _library: library(),
            context: std::ptr::null_mut(),
            destroy,
            adapter_count: adapters,
            display_info: infos,
            get_edid: invalid_edid,
            ddc_access: ddc,
        });
        let displays = provider.enumerate(&mut |_| {}).unwrap();
        let indices: Vec<_> = displays
            .iter()
            .map(|display| {
                assert_eq!(display.edid, edid());
                let RawDisplay::Amd(raw) = &display.raw else {
                    panic!("wrong SDK")
                };
                (raw.adapter_index, raw.display_index)
            })
            .collect();
        assert_eq!(indices, [(0, 11), (0, 15), (2, 16)]);
    }

    unsafe extern "system" fn ddc(
        _: AdlContextHandle,
        adapter: i32,
        display: i32,
        _: i32,
        _: i32,
        _: i32,
        _: *mut std::ffi::c_char,
        _: *mut i32,
        _: *mut std::ffi::c_char,
    ) -> i32 {
        if matches!((adapter, display), (0, 11 | 15) | (2, 16)) {
            ADL_OK
        } else {
            -3
        }
    }

    #[test]
    fn enumeration_filters_disconnected_and_alias_outputs_and_keeps_partial_results() {
        let provider = Rc::new(AdlProvider {
            _library: library(),
            context: std::ptr::null_mut(),
            destroy,
            adapter_count: adapters,
            display_info: infos,
            get_edid,
            ddc_access: ddc,
        });
        let mut diagnostics = Vec::new();
        let mut displays = provider
            .enumerate(&mut |warning| diagnostics.push(warning))
            .unwrap();
        assert_eq!(displays.len(), 3);
        assert!(
            diagnostics
                .iter()
                .any(|warning| warning.contains("adapter 0 output 17"))
        );
        assert!(
            !diagnostics
                .iter()
                .any(|warning| { warning.contains("output 20") || warning.contains("output 21") })
        );
        let mut indices = Vec::new();
        for display in &mut displays {
            assert_eq!(display.edid, edid());
            let RawDisplay::Amd(raw) = &mut display.raw else {
                panic!("wrong SDK")
            };
            indices.push((raw.adapter_index, raw.display_index));
            raw.write_packet(&[1]).unwrap();
            raw.read_packet(&mut [0; 1]).unwrap();
        }
        assert_eq!(indices, [(0, 11), (0, 15), (2, 16)]);
    }
}
