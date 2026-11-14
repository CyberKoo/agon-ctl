use agon_osd::{Address, DdcError, DdcRequest, DdcTransport, TransportError, VcpValue};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Route {
    Standard,
    Raw(usize),
}

/// Only selection state: no I/O and no retry loop. Standard and extended
/// preferences are independent and live as long as the display object.
#[derive(Default)]
struct Routing {
    preferred: [Option<Route>; 2],
    attempt: Option<(DdcRequest, Route)>,
    next: Option<(DdcRequest, Route)>,
}

fn request_class(request: DdcRequest) -> usize {
    usize::from(matches!(request, DdcRequest::Vcp(Address::Extended { .. })))
}

impl Routing {
    fn preferred(&self, request: DdcRequest, standard: bool, raw: usize) -> Option<Route> {
        self.preferred[request_class(request)].or(
            if matches!(
                request,
                DdcRequest::Vcp(Address::Standard(_)) | DdcRequest::Capabilities
            ) && standard
            {
                Some(Route::Standard)
            } else if raw != 0 {
                Some(Route::Raw(0))
            } else {
                None
            },
        )
    }

    fn select(&self, request: DdcRequest, standard: bool, raw: usize) -> Option<Route> {
        self.next
            .filter(|(pending, _)| *pending == request)
            .map(|(_, route)| route)
            .or(self.preferred(request, standard, raw))
    }

    fn recover(
        &mut self,
        request: DdcRequest,
        error: &DdcError,
        standard: bool,
        raw: usize,
    ) -> bool {
        self.next = None;
        let attempt = self.attempt.take();
        if matches!(
            error,
            DdcError::Unsupported { .. } | DdcError::Transport(TransportError::InvalidRequest(_))
        ) {
            return false;
        }
        let Some((failed, current)) = attempt else {
            return false;
        };
        if failed != request {
            return false;
        }
        let standard = standard
            && matches!(
                request,
                DdcRequest::Vcp(Address::Standard(_)) | DdcRequest::Capabilities
            );
        // Walk the eligible ring once per recovery. The client, not the ring,
        // owns the retry budget, including retries on a single transient channel.
        let next = match current {
            Route::Standard if standard => Some(if raw != 0 { Route::Raw(0) } else { current }),
            Route::Raw(index) if index < raw => Some(if index + 1 < raw {
                Route::Raw(index + 1)
            } else if standard {
                Route::Standard
            } else {
                Route::Raw(0)
            }),
            _ => None,
        }
        .filter(|next| {
            *next != current
                || !matches!(
                    error,
                    DdcError::UnsupportedFeature { .. }
                        | DdcError::UnsupportedCapabilities
                        | DdcError::Transport(TransportError::UnsupportedFeature(_))
                )
        });
        self.next = next.map(|route| (request, route));
        next.is_some()
    }
}

/// Generic only so tests exercise the same dispatch with ordinary mock channels.
pub(crate) struct Channels<S, R> {
    standard: Option<S>,
    raw: Vec<R>,
    routing: Routing,
}

impl<S, R> Channels<S, R> {
    pub(crate) fn new(standard: Option<S>, raw: Vec<R>) -> Self {
        Self {
            standard,
            raw,
            routing: Routing::default(),
        }
    }

    pub(crate) fn merge(&mut self, mut other: Self) {
        self.raw.append(&mut other.raw);
        if self.standard.is_none() {
            self.standard = other.standard.take();
        }
    }
}

impl<S: DdcTransport, R: DdcTransport> Channels<S, R> {
    fn channel(&mut self, route: Route) -> &mut dyn DdcTransport {
        match route {
            Route::Standard => self
                .standard
                .as_mut()
                .expect("selected standard channel exists"),
            Route::Raw(index) => &mut self.raw[index],
        }
    }
}

impl<S: DdcTransport, R: DdcTransport> Channels<S, R> {
    fn read_using<T>(
        &mut self,
        request: DdcRequest,
        operation: impl FnOnce(&mut dyn DdcTransport) -> Result<T, DdcError>,
    ) -> Result<T, DdcError> {
        let route = self
            .routing
            .select(request, self.standard.is_some(), self.raw.len());
        self.routing.next = None;
        self.routing.attempt = None;
        let route = route.ok_or_else(|| request.unsupported_error())?;
        self.routing.attempt = Some((request, route));
        // Delegate the whole transaction, including decode, to exactly one channel.
        let result = operation(self.channel(route));
        if result.is_ok() {
            self.routing.preferred[request_class(request)] = Some(route);
            self.routing.attempt = None;
        }
        result
    }
}

impl<S: DdcTransport, R: DdcTransport> DdcTransport for Channels<S, R> {
    fn read_vcp(&mut self, address: Address) -> Result<VcpValue, DdcError> {
        self.read_using(DdcRequest::Vcp(address), |channel| {
            channel.read_vcp(address)
        })
    }

    fn write_vcp(&mut self, address: Address, value: u16) -> Result<(), DdcError> {
        self.routing.attempt = None;
        self.routing.next = None;
        let route = self
            .routing
            .preferred(
                DdcRequest::Vcp(address),
                self.standard.is_some(),
                self.raw.len(),
            )
            .ok_or(DdcError::UnsupportedFeature { address })?;
        // A failed write may already have reached the monitor. Never fall back.
        self.channel(route).write_vcp(address, value)
    }

    fn read_capabilities(&mut self, offset: u16) -> Result<agon_osd::CapabilityFragment, DdcError> {
        self.read_using(DdcRequest::Capabilities, |channel| {
            channel.read_capabilities(offset)
        })
    }

    fn recover_read(&mut self, address: Address, error: &DdcError) -> bool {
        if self.routing.attempt == Some((DdcRequest::Vcp(address), Route::Standard))
            && crate::dxva2::is_refreshed_monitor_error(error)
        {
            self.routing.next = Some((DdcRequest::Vcp(address), Route::Standard));
            return true;
        }
        self.routing.recover(
            DdcRequest::Vcp(address),
            error,
            self.standard.is_some(),
            self.raw.len(),
        )
    }

    fn recover_capabilities(&mut self, error: &DdcError) -> bool {
        self.routing.recover(
            DdcRequest::Capabilities,
            error,
            self.standard.is_some(),
            self.raw.len(),
        )
    }
}

#[cfg(test)]
mod tests;
