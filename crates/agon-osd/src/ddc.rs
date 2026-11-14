use crate::{Address, CapabilityFragment, DdcError, DdcRequest, DdcTransport, VcpValue};
use std::thread;
use std::time::Duration;

const MAX_ATTEMPTS: usize = 3;

const COMMAND_GAP: Duration = Duration::from_millis(50);
const WRITE_DELAY: Duration = Duration::from_millis(100);
const RETRY_DELAY: Duration = Duration::from_millis(20);

pub(crate) struct DdcClient<T> {
    transport: T,
}

impl<T: DdcTransport> DdcClient<T> {
    pub(crate) fn new(transport: T) -> Self {
        Self { transport }
    }

    pub(crate) fn into_inner(self) -> T {
        self.transport
    }

    pub(crate) fn read(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        self.read_with_retry(DdcRequest::Vcp(address), |transport| {
            transport.read_vcp(address)
        })
    }

    pub(crate) fn read_capabilities(
        &mut self,
        offset: u16,
    ) -> Result<CapabilityFragment, DdcError> {
        self.read_with_retry(DdcRequest::Capabilities, |transport| {
            transport.read_capabilities(offset)
        })
    }

    fn read_with_retry<R>(
        &mut self,
        request: DdcRequest,
        mut operation: impl FnMut(&mut T) -> Result<R, DdcError>,
    ) -> Result<R, DdcError> {
        let mut attempt = 1;
        loop {
            let result = operation(&mut self.transport);

            match result {
                Ok(value) => {
                    thread::sleep(COMMAND_GAP);
                    return Ok(value);
                }
                Err(error) => {
                    let recover = match request {
                        DdcRequest::Vcp(address) => self.transport.recover_read(address, &error),
                        DdcRequest::Capabilities => self.transport.recover_capabilities(&error),
                    };
                    if attempt == MAX_ATTEMPTS || !recover {
                        thread::sleep(COMMAND_GAP);
                        return Err(error);
                    }
                    attempt += 1;
                    thread::sleep(RETRY_DELAY);
                }
            }
        }
    }

    pub(crate) fn write(&mut self, address: Address, value: u16) -> Result<(), DdcError> {
        // An I/O error does not prove that the display missed the command.
        // Retrying here could repeat an input switch or a panel maintenance action.
        let result = self.transport.write_vcp(address, value);
        thread::sleep(WRITE_DELAY);
        result
    }
}
