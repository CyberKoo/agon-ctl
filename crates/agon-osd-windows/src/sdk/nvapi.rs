use super::{DiscoveredDisplay, RawDisplay};
use crate::{Library, WindowsError};
use agon_osd::{DisplayInfo, PacketTransport, TransportError, parse_edid};
use agon_osd_windows_sys::nvapi::*;
use std::rc::Rc;

const PROVIDER: &str = "NVIDIA NvAPI";

pub(crate) struct NvProvider {
    _library: Rc<Library>,
    enum_gpus: NvEnumPhysicalGpus,
    connected_displays: NvGpuGetConnectedDisplayIds,
    gpu_and_output: NvSysGetGpuAndOutputIdFromDisplayId,
    get_edid: NvGpuGetEdid,
    i2c_read: NvI2cRead,
    i2c_write: NvI2cWrite,
}

impl NvProvider {
    pub(crate) fn new() -> Result<Rc<Self>, WindowsError> {
        let library = Rc::new(Library::load(PROVIDER, &["nvapi64.dll", "nvapi.dll"])?);
        let query: NvQueryInterface = library.symbol(c"nvapi_QueryInterface")?;
        let initialize = interface::<NvInitialize>(query, NVAPI_INITIALIZE, "NvAPI_Initialize")?;
        let enum_gpus = interface::<NvEnumPhysicalGpus>(
            query,
            NVAPI_ENUM_PHYSICAL_GPUS,
            "NvAPI_EnumPhysicalGPUs",
        )?;
        let connected_displays = interface::<NvGpuGetConnectedDisplayIds>(
            query,
            NVAPI_GPU_GET_CONNECTED_DISPLAY_IDS,
            "NvAPI_GPU_GetConnectedDisplayIds",
        )?;
        let gpu_and_output = interface::<NvSysGetGpuAndOutputIdFromDisplayId>(
            query,
            NVAPI_SYS_GET_GPU_AND_OUTPUT_ID_FROM_DISPLAY_ID,
            "NvAPI_SYS_GetGpuAndOutputIdFromDisplayId",
        )?;
        let get_edid = interface::<NvGpuGetEdid>(query, NVAPI_GPU_GET_EDID, "NvAPI_GPU_GetEDID")?;
        let i2c_read = interface::<NvI2cRead>(query, NVAPI_I2C_READ, "NvAPI_I2CRead")?;
        let i2c_write = interface::<NvI2cWrite>(query, NVAPI_I2C_WRITE, "NvAPI_I2CWrite")?;
        // SAFETY: the initialization function was returned by nvapi_QueryInterface.
        let status = unsafe { initialize() };
        check(status, "NvAPI_Initialize")?;
        Ok(Rc::new(Self {
            _library: library,
            enum_gpus,
            connected_displays,
            gpu_and_output,
            get_edid,
            i2c_read,
            i2c_write,
        }))
    }

    pub(crate) fn enumerate(
        self: &Rc<Self>,
        diagnostic: &mut dyn FnMut(String),
    ) -> Result<Vec<DiscoveredDisplay>, WindowsError> {
        let mut gpus = [std::ptr::null_mut(); NVAPI_MAX_PHYSICAL_GPUS];
        let mut count = 0;
        // SAFETY: the GPU array has NVAPI_MAX_PHYSICAL_GPUS writable entries.
        let status = unsafe { (self.enum_gpus)(gpus.as_mut_ptr(), &mut count) };
        check(status, "NvAPI_EnumPhysicalGPUs")?;
        let mut displays = Vec::new();
        for gpu in gpus.into_iter().take(count as usize) {
            let mut display_count = 0;
            // SAFETY: a null list queries the number of connected display IDs.
            let status = unsafe {
                (self.connected_displays)(gpu, std::ptr::null_mut(), &mut display_count, 0)
            };
            if let Err(error) = check(status, "NvAPI_GPU_GetConnectedDisplayIds(count)") {
                diagnostic(format!("{PROVIDER} GPU {gpu:p}: {error}"));
                continue;
            }
            let mut ids = vec![
                NvGpuDisplayIds {
                    version: (0x10000 + std::mem::size_of::<NvGpuDisplayIds>()) as u32,
                    ..Default::default()
                };
                display_count as usize
            ];
            if display_count > 0 {
                // SAFETY: `ids` has capacity for `display_count` structures.
                let status = unsafe {
                    (self.connected_displays)(gpu, ids.as_mut_ptr(), &mut display_count, 0)
                };
                if let Err(error) = check(status, "NvAPI_GPU_GetConnectedDisplayIds(list)") {
                    diagnostic(format!("{PROVIDER} GPU {gpu:p}: {error}"));
                    continue;
                }
            }
            ids.truncate(display_count as usize);
            for id in ids {
                let (output_gpu, output_id) = match self.resolve_output(id.display_id) {
                    Ok(output) => output,
                    Err(error) => {
                        diagnostic(format!("{PROVIDER} display {}: {error}", id.display_id));
                        continue;
                    }
                };
                let mut edid = NvEdid::default();
                // SAFETY: use the converted GPU/output pair, not the opaque display ID.
                let status = unsafe { (self.get_edid)(output_gpu, output_id, &mut edid) };
                if let Err(error) = check(status, "NvAPI_GPU_GetEDID") {
                    diagnostic(format!("{PROVIDER} display {}: {error}", id.display_id));
                    continue;
                }
                let size = (edid.sizeof_edid as usize).min(edid.edid_data.len());
                let Some(identity) = parse_edid(&edid.edid_data[..size]) else {
                    diagnostic(format!(
                        "{PROVIDER} display {}: invalid EDID base block",
                        id.display_id
                    ));
                    continue;
                };
                let mut key = [0; 128];
                key.copy_from_slice(&edid.edid_data[..128]);
                let location = format!("nvapi:{:p}:{}", gpu, id.display_id);
                let info = DisplayInfo::new(
                    location.clone(),
                    identity.name.unwrap_or_else(|| location.clone()),
                    identity.vendor_id,
                    identity.model_id,
                    identity.serial_number,
                    location,
                );
                displays.push(DiscoveredDisplay {
                    info,
                    edid: key,
                    raw: RawDisplay::Nvidia(NvDisplay {
                        provider: Rc::clone(self),
                        gpu: output_gpu,
                        output_id,
                    }),
                });
            }
        }
        Ok(displays)
    }

    fn resolve_output(&self, display_id: u32) -> Result<(NvPhysicalGpuHandle, u32), WindowsError> {
        let mut gpu = std::ptr::null_mut();
        let mut output_id = 0;
        // SAFETY: both output pointers are writable and the signature matches nvapi.h.
        let status = unsafe { (self.gpu_and_output)(display_id, &mut gpu, &mut output_id) };
        check(status, "NvAPI_SYS_GetGpuAndOutputIdFromDisplayId")?;
        if gpu.is_null() || !output_id.is_power_of_two() {
            return Err(WindowsError::InvalidData(
                "NvAPI returned an invalid GPU/output pair",
            ));
        }
        Ok((gpu, output_id))
    }
}

pub(crate) struct NvDisplay {
    provider: Rc<NvProvider>,
    gpu: NvPhysicalGpuHandle,
    output_id: u32,
}

impl NvDisplay {
    fn request(&mut self, address: u8, data: *mut u8, length: usize) -> NvI2cInfoV3 {
        NvI2cInfoV3 {
            version: (0x30000 + std::mem::size_of::<NvI2cInfoV3>()) as u32,
            display_mask: self.output_id,
            is_ddc_port: 1,
            i2c_dev_address: address,
            i2c_reg_address: std::ptr::null_mut(),
            reg_addr_size: 0,
            data,
            data_size: length as u32,
            i2c_speed: NVAPI_I2C_SPEED_DEPRECATED,
            i2c_speed_khz: 0,
            port_id: 0,
            is_port_id_set: 0,
        }
    }
}

impl PacketTransport for NvDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        let mut request = self.request(0x6E, packet.as_ptr() as *mut u8, packet.len());
        // SAFETY: the request follows NV_I2C_INFO_V3 and the packet remains alive for the call.
        let status = unsafe { (self.provider.i2c_write)(self.gpu, &mut request) };
        if status == NVAPI_OK {
            Ok(())
        } else {
            Err(TransportError::Io {
                operation: "NvAPI_I2CWrite",
                code: status,
            })
        }
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        let mut request = self.request(0x6F, response.as_mut_ptr(), response.len());
        // SAFETY: the request follows NV_I2C_INFO_V3 and the output buffer has the requested capacity.
        let status = unsafe { (self.provider.i2c_read)(self.gpu, &mut request) };
        if status != NVAPI_OK {
            return Err(TransportError::Io {
                operation: "NvAPI_I2CRead",
                code: status,
            });
        }
        let actual = request.data_size as usize;
        if actual > response.len() {
            return Err(TransportError::UnexpectedEof("NvAPI_I2CRead"));
        }
        Ok(())
    }
}

fn interface<T: Copy>(
    query: NvQueryInterface,
    id: u32,
    name: &'static str,
) -> Result<T, WindowsError> {
    // SAFETY: NvAPI interface IDs are stable, and null is checked before converting the pointer.
    let address = unsafe { query(id) };
    if address.is_null() {
        return Err(WindowsError::MissingSymbol {
            provider: PROVIDER,
            symbol: name,
        });
    }
    // SAFETY: the caller chooses T to match the interface ID's documented function signature.
    Ok(unsafe { std::mem::transmute_copy(&address) })
}

fn check(status: NvStatus, operation: &'static str) -> Result<(), WindowsError> {
    if status == NVAPI_OK {
        Ok(())
    } else {
        Err(WindowsError::Call {
            provider: PROVIDER,
            operation,
            code: status,
        })
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::super::tests::{edid, library};
    use super::*;

    unsafe extern "C" fn enum_gpus(gpus: *mut NvPhysicalGpuHandle, count: *mut u32) -> NvStatus {
        // SAFETY: the caller provides the standard GPU array and count outputs.
        unsafe {
            *count = 4;
            for index in 0..4 {
                *gpus.add(index) = (index + 1) as NvPhysicalGpuHandle;
            }
        }
        NVAPI_OK
    }

    unsafe extern "C" fn connected(
        gpu: NvPhysicalGpuHandle,
        ids: *mut NvGpuDisplayIds,
        count: *mut u32,
        _: u32,
    ) -> NvStatus {
        if gpu as usize == 2 || (gpu as usize == 3 && !ids.is_null()) {
            return -1;
        }
        let values: &[u32] = if gpu as usize == 1 {
            &[1001, 1002, 1003, 1004, 1005, 1006]
        } else {
            &[1007]
        };
        // SAFETY: the count query precedes the list call, which provides enough entries.
        unsafe {
            *count = values.len() as u32;
            if !ids.is_null() {
                for (index, value) in values.iter().enumerate() {
                    (*ids.add(index)).display_id = *value;
                }
            }
        }
        NVAPI_OK
    }

    unsafe extern "C" fn convert(
        id: u32,
        gpu: *mut NvPhysicalGpuHandle,
        output: *mut u32,
    ) -> NvStatus {
        if id == 1002 {
            return -2;
        }
        // SAFETY: writable output arguments are provided by resolve_output.
        unsafe {
            *gpu = if id == 1008 {
                std::ptr::null_mut()
            } else {
                9_usize as NvPhysicalGpuHandle
            };
            *output = match id {
                1003 => 0,
                1004 => 3,
                _ => 1 << (id - 1001),
            };
        }
        NVAPI_OK
    }

    unsafe extern "C" fn get_edid(
        gpu: NvPhysicalGpuHandle,
        output: u32,
        data: *mut NvEdid,
    ) -> NvStatus {
        if gpu as usize != 9 || !output.is_power_of_two() || output == 16 {
            return -3;
        }
        // SAFETY: the caller supplies an initialized, writable NvEdid.
        unsafe {
            (*data).sizeof_edid = 512; // Extensions are not needed for the base identity.
            (&mut (*data).edid_data)[..128].copy_from_slice(&edid());
        }
        NVAPI_OK
    }

    unsafe extern "C" fn i2c(gpu: NvPhysicalGpuHandle, request: *mut NvI2cInfoV3) -> NvStatus {
        // SAFETY: the transport supplies a valid request for this synchronous call.
        let mask = unsafe { (*request).display_mask };
        if gpu as usize == 9 && matches!(mask, 1 | 32 | 64) {
            NVAPI_OK
        } else {
            -4
        }
    }

    fn provider() -> Rc<NvProvider> {
        Rc::new(NvProvider {
            _library: library(),
            enum_gpus,
            connected_displays: connected,
            gpu_and_output: convert,
            get_edid,
            i2c_read: i2c,
            i2c_write: i2c,
        })
    }

    #[test]
    fn enumeration_keeps_outputs_across_conversion_edid_and_adapter_errors() {
        let mut displays = provider().enumerate(&mut |_| {}).unwrap();
        assert_eq!(displays.len(), 3);
        for display in &mut displays {
            assert_eq!(display.edid, edid());
            let RawDisplay::Nvidia(raw) = &mut display.raw else {
                panic!("wrong SDK")
            };
            assert_eq!(raw.gpu as usize, 9);
            assert!(matches!(raw.output_id, 1 | 32 | 64));
            raw.write_packet(&[1]).unwrap();
            raw.read_packet(&mut [0; 1]).unwrap();
        }
    }

    #[test]
    fn rejects_null_gpu_and_preserves_global_enumeration_errors() {
        let mut provider = provider();
        assert!(matches!(
            provider.resolve_output(1008),
            Err(WindowsError::InvalidData(_))
        ));
        unsafe extern "C" fn fail(_: *mut NvPhysicalGpuHandle, _: *mut u32) -> NvStatus {
            -77
        }
        Rc::get_mut(&mut provider).unwrap().enum_gpus = fail;
        assert!(matches!(
            provider.enumerate(&mut |_| {}),
            Err(WindowsError::Call { code: -77, .. })
        ));
    }

    unsafe extern "C" fn query(id: u32) -> *mut std::ffi::c_void {
        // Literal from NVIDIA's nvapi_interface.h, independent of our binding constant.
        if id == 0x112ba1a5 {
            convert as *const () as *mut std::ffi::c_void
        } else {
            std::ptr::null_mut()
        }
    }

    #[test]
    fn official_conversion_interface_id_resolves_with_cdecl_abi() {
        let convert = interface::<NvSysGetGpuAndOutputIdFromDisplayId>(
            query,
            NVAPI_SYS_GET_GPU_AND_OUTPUT_ID_FROM_DISPLAY_ID,
            "NvAPI_SYS_GetGpuAndOutputIdFromDisplayId",
        )
        .unwrap();
        let mut gpu = std::ptr::null_mut();
        let mut output = 0;
        // SAFETY: the test query returns a function matching the official C ABI.
        assert_eq!(unsafe { convert(1006, &mut gpu, &mut output) }, NVAPI_OK);
        assert_eq!(gpu as usize, 9);
        assert_eq!(output, 32);
    }

    unsafe extern "C" fn missing(_: u32) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    #[test]
    fn missing_conversion_interface_is_not_an_operation_error() {
        let result = interface::<NvSysGetGpuAndOutputIdFromDisplayId>(
            missing,
            NVAPI_SYS_GET_GPU_AND_OUTPUT_ID_FROM_DISPLAY_ID,
            "NvAPI_SYS_GetGpuAndOutputIdFromDisplayId",
        );
        assert!(matches!(result, Err(WindowsError::MissingSymbol { .. })));
        assert!(matches!(
            check(-1, "operation"),
            Err(WindowsError::Call { code: -1, .. })
        ));
    }
}
