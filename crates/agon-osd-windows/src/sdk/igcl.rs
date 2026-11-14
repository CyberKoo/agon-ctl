use super::{DiscoveredDisplay, RawDisplay};
use crate::{Library, WindowsError};
use agon_osd::{DisplayInfo, PacketTransport, TransportError, parse_edid};
use agon_osd_windows_sys::igcl::*;
use std::rc::Rc;

const PROVIDER: &str = "Intel IGCL";

pub(crate) struct IgclProvider {
    _library: Rc<Library>,
    api: CtlApiHandle,
    close: CtlClose,
    enumerate_devices: CtlEnumerateDevices,
    enumerate_outputs: CtlEnumerateDisplayOutputs,
    i2c_access: CtlI2cAccess,
    panel_access: CtlPanelDescriptorAccess,
}

impl IgclProvider {
    pub(crate) fn new() -> Result<Rc<Self>, WindowsError> {
        let library = Rc::new(Library::load(
            PROVIDER,
            &["ControlLib.dll", "ControlLib32.dll"],
        )?);
        let init: CtlInit = library.symbol(c"ctlInit")?;
        let close = library.symbol(c"ctlClose")?;
        let enumerate_devices = library.symbol(c"ctlEnumerateDevices")?;
        let enumerate_outputs = library.symbol(c"ctlEnumerateDisplayOutputs")?;
        let i2c_access = library.symbol(c"ctlI2CAccess")?;
        let panel_access = library.symbol(c"ctlPanelDescriptorAccess")?;
        let mut args = CtlInitArgs {
            size: std::mem::size_of::<CtlInitArgs>() as u32,
            version: 0,
            app_version: CTL_IMPL_VERSION,
            flags: 0,
            supported_version: 0,
            application_uid: CtlApplicationId::default(),
        };
        let mut api = std::ptr::null_mut();
        // SAFETY: all function pointers were resolved from the loaded IGCL and the arguments are valid.
        let status = unsafe { init(&mut args, &mut api) };
        if status != 0 {
            return Err(WindowsError::Call {
                provider: PROVIDER,
                operation: "ctlInit",
                code: status,
            });
        }
        if api.is_null() {
            return Err(WindowsError::InvalidData("IGCL returned a null API handle"));
        }
        Ok(Rc::new(Self {
            _library: library,
            api,
            close,
            enumerate_devices,
            enumerate_outputs,
            i2c_access,
            panel_access,
        }))
    }

    pub(crate) fn enumerate(
        self: &Rc<Self>,
        diagnostic: &mut dyn FnMut(String),
    ) -> Result<Vec<DiscoveredDisplay>, WindowsError> {
        let mut count = 0;
        // SAFETY: the API handle is valid and the count pointer is writable.
        let status =
            unsafe { (self.enumerate_devices)(self.api, &mut count, std::ptr::null_mut()) };
        check(status, "ctlEnumerateDevices(count)")?;
        let mut devices = vec![std::ptr::null_mut(); count as usize];
        if count > 0 {
            // SAFETY: `devices` has capacity for `count` handles and the count remains writable.
            let status =
                unsafe { (self.enumerate_devices)(self.api, &mut count, devices.as_mut_ptr()) };
            check(status, "ctlEnumerateDevices(list)")?;
        }
        devices.truncate(count as usize);

        let mut displays = Vec::new();
        for device in devices {
            let mut output_count = 0;
            // SAFETY: the adapter handle comes from IGCL and the count pointer is writable.
            let status = unsafe {
                (self.enumerate_outputs)(device, &mut output_count, std::ptr::null_mut())
            };
            if let Err(error) = check(status, "ctlEnumerateDisplayOutputs(count)") {
                diagnostic(format!("{PROVIDER} adapter {device:p}: {error}"));
                continue;
            }
            let mut outputs = vec![std::ptr::null_mut(); output_count as usize];
            if output_count > 0 {
                // SAFETY: `outputs` has capacity for `output_count` handles.
                let status = unsafe {
                    (self.enumerate_outputs)(device, &mut output_count, outputs.as_mut_ptr())
                };
                if let Err(error) = check(status, "ctlEnumerateDisplayOutputs(list)") {
                    diagnostic(format!("{PROVIDER} adapter {device:p}: {error}"));
                    continue;
                }
            }
            outputs.truncate(output_count as usize);
            for output in outputs {
                let edid = match self.read_edid(output, diagnostic) {
                    Ok(Some(edid)) => edid,
                    Ok(None) => {
                        diagnostic(format!(
                            "{PROVIDER} output {output:p}: missing EDID base block"
                        ));
                        continue;
                    }
                    Err(error) => {
                        diagnostic(format!("{PROVIDER} output {output:p}: {error}"));
                        continue;
                    }
                };
                let mut key = [0; 128];
                key.copy_from_slice(&edid[..128]);
                let Some(identity) = parse_edid(&edid) else {
                    diagnostic(format!(
                        "{PROVIDER} output {output:p}: invalid EDID base block"
                    ));
                    continue;
                };
                let location = format!("igcl:{output:p}");
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
                    raw: RawDisplay::Intel(IgclDisplay {
                        provider: Rc::clone(self),
                        output,
                    }),
                });
            }
        }
        Ok(displays)
    }

    fn read_edid(
        &self,
        output: CtlDisplayOutputHandle,
        diagnostic: &mut dyn FnMut(String),
    ) -> Result<Option<Vec<u8>>, WindowsError> {
        let mut edid = self.read_edid_block(output, 0)?;
        let Some(first) = edid.take() else {
            return Ok(None);
        };
        if first.len() < 128 {
            return Ok(None);
        }
        let extension_count = first[126];
        let mut result = first;
        if extension_count > 0 {
            match self.read_edid_block(output, 1) {
                Ok(Some(extension)) if extension.len() >= 128 => result.extend(extension),
                Ok(_) => diagnostic(format!(
                    "{PROVIDER} output {output:p}: missing/truncated EDID extension"
                )),
                Err(error) => diagnostic(format!(
                    "{PROVIDER} output {output:p}: EDID extension: {error}"
                )),
            }
        }
        Ok(Some(result))
    }

    fn read_edid_block(
        &self,
        output: CtlDisplayOutputHandle,
        block: u32,
    ) -> Result<Option<Vec<u8>>, WindowsError> {
        let mut request = CtlPanelDescriptorAccessArgs {
            size: std::mem::size_of::<CtlPanelDescriptorAccessArgs>() as u32,
            version: 0,
            operation_type: CTL_OPERATION_TYPE_READ,
            block_number: block,
            descriptor_data_size: 0,
            descriptor_data: std::ptr::null_mut(),
        };
        // SAFETY: the output handle and request are valid; a null buffer queries the required size.
        let status = unsafe { (self.panel_access)(output, &mut request) };
        check(status, "ctlPanelDescriptorAccess(size)")?;
        let size = request.descriptor_data_size as usize;
        if size == 0 {
            return Ok(None);
        }
        if size > 256 {
            return Err(WindowsError::InvalidData(
                "IGCL returned an oversized EDID block",
            ));
        }
        let mut data = vec![0; size];
        request.descriptor_data = data.as_mut_ptr();
        // SAFETY: the output buffer has `descriptor_data_size` writable bytes.
        let status = unsafe { (self.panel_access)(output, &mut request) };
        check(status, "ctlPanelDescriptorAccess(data)")?;
        let actual = request.descriptor_data_size as usize;
        if actual > data.len() {
            return Err(WindowsError::InvalidData(
                "IGCL returned an oversized EDID result",
            ));
        }
        data.truncate(actual);
        Ok(Some(data))
    }
}

impl Drop for IgclProvider {
    fn drop(&mut self) {
        // SAFETY: the API handle is valid until all dependent displays release the provider.
        unsafe { (self.close)(self.api) };
    }
}

pub(crate) struct IgclDisplay {
    provider: Rc<IgclProvider>,
    output: CtlDisplayOutputHandle,
}

impl PacketTransport for IgclDisplay {
    fn write_packet(&mut self, packet: &[u8]) -> Result<(), TransportError> {
        if packet.len() < 2 || packet.len() - 1 > CTL_I2C_MAX_DATA_SIZE {
            return Err(TransportError::InvalidRequest(
                "invalid IGCL DDC packet length",
            ));
        }
        let mut request = CtlI2cAccessArgs {
            data_size: (packet.len() - 1) as u32,
            address: 0x6E,
            operation_type: CTL_OPERATION_TYPE_WRITE,
            offset: 0x51,
            ..Default::default()
        };
        request.data[..packet.len() - 1].copy_from_slice(&packet[1..]);
        // SAFETY: the display handle is valid and the request contains a complete payload.
        let status = unsafe { (self.provider.i2c_access)(self.output, &mut request) };
        if status == 0 {
            Ok(())
        } else {
            Err(TransportError::Io {
                operation: "ctlI2CAccess(write)",
                code: status,
            })
        }
    }

    fn read_packet(&mut self, response: &mut [u8]) -> Result<(), TransportError> {
        if response.len() > CTL_I2C_MAX_DATA_SIZE {
            return Err(TransportError::InvalidRequest(
                "IGCL response buffer is too large",
            ));
        }
        let mut request = CtlI2cAccessArgs {
            data_size: response.len() as u32,
            // IGCL reads use the DDC read path with a zero offset so the driver returns
            // the complete response frame, including the leading 0x6E byte.
            address: 0x6E,
            operation_type: CTL_OPERATION_TYPE_READ,
            offset: 0,
            ..Default::default()
        };
        // SAFETY: the display handle is valid and the response buffer has the requested capacity.
        let status = unsafe { (self.provider.i2c_access)(self.output, &mut request) };
        if status != 0 {
            return Err(TransportError::Io {
                operation: "ctlI2CAccess(read)",
                code: status,
            });
        }
        let actual = request.data_size as usize;
        if actual > response.len() {
            return Err(TransportError::UnexpectedEof("ctlI2CAccess(read)"));
        }
        response[..actual].copy_from_slice(&request.data[..actual]);
        Ok(())
    }
}

fn check(status: CtlResult, operation: &'static str) -> Result<(), WindowsError> {
    if status == 0 {
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

    unsafe extern "C" fn close(_: CtlApiHandle) -> CtlResult {
        0
    }
    unsafe extern "C" fn i2c(_: CtlDisplayOutputHandle, _: *mut CtlI2cAccessArgs) -> CtlResult {
        0
    }

    unsafe extern "C" fn devices(
        _: CtlApiHandle,
        count: *mut u32,
        handles: *mut CtlDeviceAdapterHandle,
    ) -> CtlResult {
        // SAFETY: the caller queries the count before providing an array of that length.
        unsafe {
            *count = 4;
            if !handles.is_null() {
                for index in 0..4 {
                    *handles.add(index) = (index + 1) as CtlDeviceAdapterHandle;
                }
            }
        }
        0
    }

    unsafe extern "C" fn outputs(
        device: CtlDeviceAdapterHandle,
        count: *mut u32,
        handles: *mut CtlDisplayOutputHandle,
    ) -> CtlResult {
        if device as usize == 2 || (device as usize == 3 && !handles.is_null()) {
            return -1;
        }
        let values: &[usize] = if device as usize == 1 {
            &[101, 102, 103, 104, 105]
        } else {
            &[106]
        };
        // SAFETY: the list buffer has the size returned by the count query.
        unsafe {
            *count = values.len() as u32;
            if !handles.is_null() {
                for (index, value) in values.iter().enumerate() {
                    *handles.add(index) = *value as CtlDisplayOutputHandle;
                }
            }
        }
        0
    }

    unsafe extern "C" fn panel(
        output: CtlDisplayOutputHandle,
        request: *mut CtlPanelDescriptorAccessArgs,
    ) -> CtlResult {
        let id = output as usize;
        // SAFETY: the caller supplies a valid request and a buffer of the queried size.
        unsafe {
            let request = &mut *request;
            if id == 102 || (request.block_number == 1 && id == 101) {
                return -2;
            }
            if request.descriptor_data.is_null() {
                request.descriptor_data_size = if id == 108 && request.block_number == 1 {
                    257
                } else {
                    128
                };
            } else {
                if id == 107 && request.block_number == 1 {
                    return -3;
                }
                let mut block = edid();
                if id == 103 {
                    block[0] = 1;
                }
                std::ptr::copy_nonoverlapping(block.as_ptr(), request.descriptor_data, 128);
                if id == 104 || (id == 105 && request.block_number == 1) {
                    request.descriptor_data_size = 64;
                }
                if id == 109 && request.block_number == 1 {
                    request.descriptor_data_size = 129;
                }
            }
        }
        0
    }

    fn provider() -> Rc<IgclProvider> {
        Rc::new(IgclProvider {
            _library: library(),
            api: std::ptr::null_mut(),
            close,
            enumerate_devices: devices,
            enumerate_outputs: outputs,
            i2c_access: i2c,
            panel_access: panel,
        })
    }

    #[test]
    fn enumeration_keeps_base_edid_across_output_extension_and_adapter_errors() {
        let provider = provider();
        let displays = provider.enumerate(&mut |_| {}).unwrap();
        assert_eq!(displays.len(), 3);
        let outputs: Vec<_> = displays
            .iter()
            .map(|display| {
                assert_eq!(display.edid, edid());
                let RawDisplay::Intel(raw) = &display.raw else {
                    panic!("wrong SDK")
                };
                raw.output as usize
            })
            .collect();
        assert_eq!(outputs, [101, 105, 106]);
        assert_eq!(
            provider
                .read_edid(101_usize as CtlDisplayOutputHandle, &mut |_| {})
                .unwrap()
                .unwrap(),
            edid()
        );
        assert_eq!(
            provider
                .read_edid(105_usize as CtlDisplayOutputHandle, &mut |_| {})
                .unwrap()
                .unwrap(),
            edid()
        );
    }

    #[test]
    fn extension_data_and_size_errors_preserve_the_base_block() {
        let provider = provider();
        for id in [107_usize, 108, 109] {
            assert_eq!(
                provider
                    .read_edid(id as CtlDisplayOutputHandle, &mut |_| {})
                    .unwrap()
                    .unwrap(),
                edid()
            );
        }
    }

    #[test]
    fn global_enumeration_errors_are_not_reported_as_empty_success() {
        unsafe extern "C" fn fail(
            _: CtlApiHandle,
            _: *mut u32,
            _: *mut CtlDeviceAdapterHandle,
        ) -> CtlResult {
            -77
        }
        let mut provider = provider();
        Rc::get_mut(&mut provider).unwrap().enumerate_devices = fail;
        assert!(matches!(
            provider.enumerate(&mut |_| {}),
            Err(WindowsError::Call { code: -77, .. })
        ));
    }

    #[test]
    fn operation_errors_remain_distinct_from_missing_sdk() {
        assert!(matches!(
            check(-7, "ctlPanelDescriptorAccess"),
            Err(WindowsError::Call { code: -7, .. })
        ));
    }
}
