//! Synchronous requests to a dedicated, thread-confined device worker.

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows", test))]
mod implementation {
    use crate::backend::Backend;
    use crate::error::ApiError;
    use agon_osd::{DisplayInfo, Feature, VcpValue};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use std::thread::{self, JoinHandle};

    const QUEUE_CAPACITY: usize = 16;
    type Reply<T> = SyncSender<Result<T, ApiError>>;

    enum Command {
        Refresh(Reply<Vec<DisplayInfo>>),
        Read(String, Feature, Reply<VcpValue>),
        Set(String, Feature, u16, Reply<VcpValue>),
        Probe(String, Feature, Reply<Option<VcpValue>>),
        Capabilities(String, Feature, Reply<Vec<u8>>),
    }

    pub(crate) struct Session {
        sender: Option<SyncSender<Command>>,
        worker: Option<JoinHandle<()>>,
    }

    impl Session {
        pub(crate) fn new() -> Result<Self, ApiError> {
            #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
            {
                Self::with_factory(crate::backend::NativeBackend::new)
            }
            #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
            {
                Err(ApiError::new(
                    crate::error::UNSUPPORTED_PLATFORM,
                    "display control supports macOS, Linux and Windows",
                ))
            }
        }

        // Only the factory crosses the thread boundary, never the backend it creates.
        pub(crate) fn with_factory<B, F>(factory: F) -> Result<Self, ApiError>
        where
            B: Backend,
            F: FnOnce() -> Result<B, ApiError> + Send + 'static,
        {
            let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
            let (ready, initialized) = mpsc::sync_channel(1);
            let worker = thread::Builder::new()
                .name("agon-osd".into())
                .spawn(move || {
                    // A panic terminates the worker, disconnecting all pending replies.
                    // In particular, a partially mutated backend is never used again.
                    let _ = catch_unwind(AssertUnwindSafe(|| run(factory, receiver, ready)));
                })
                .map_err(|error| {
                    ApiError::internal(format!("could not start display worker: {error}"))
                })?;
            let session = Self {
                sender: Some(sender),
                worker: Some(worker),
            };
            initialized
                .recv()
                .map_err(|_| ApiError::internal("display worker failed during initialization"))??;
            Ok(session)
        }

        fn request<T>(&self, command: impl FnOnce(Reply<T>) -> Command) -> Result<T, ApiError> {
            let (reply, receiver) = mpsc::sync_channel(1);
            self.sender
                .as_ref()
                .ok_or_else(worker_stopped)?
                .send(command(reply))
                .map_err(|_| worker_stopped())?;
            receiver.recv().map_err(|_| worker_stopped())?
        }

        pub(crate) fn refresh(&self) -> Result<Vec<DisplayInfo>, ApiError> {
            self.request(Command::Refresh)
        }

        pub(crate) fn read(&self, id: String, feature: Feature) -> Result<VcpValue, ApiError> {
            self.request(|reply| Command::Read(id, feature, reply))
        }

        pub(crate) fn set(
            &self,
            id: String,
            feature: Feature,
            value: u16,
        ) -> Result<VcpValue, ApiError> {
            self.request(|reply| Command::Set(id, feature, value, reply))
        }

        pub(crate) fn probe(
            &self,
            id: String,
            feature: Feature,
        ) -> Result<Option<VcpValue>, ApiError> {
            self.request(|reply| Command::Probe(id, feature, reply))
        }

        pub(crate) fn capabilities(
            &self,
            id: String,
            feature: Feature,
        ) -> Result<Vec<u8>, ApiError> {
            self.request(|reply| Command::Capabilities(id, feature, reply))
        }
    }

    fn worker_stopped() -> ApiError {
        ApiError::internal("display worker stopped unexpectedly")
    }

    fn run<B: Backend>(
        factory: impl FnOnce() -> Result<B, ApiError>,
        receiver: Receiver<Command>,
        ready: Reply<()>,
    ) {
        let mut backend = match factory() {
            Ok(backend) => backend,
            Err(error) => {
                let _ = ready.send(Err(error));
                return;
            }
        };
        if ready.send(Ok(())).is_err() {
            return;
        }
        while let Ok(command) = receiver.recv() {
            match command {
                Command::Refresh(reply) => {
                    let _ = reply.send(backend.refresh());
                }
                Command::Read(id, feature, reply) => {
                    let _ = reply.send(backend.read(&id, feature));
                }
                Command::Set(id, feature, value, reply) => {
                    let _ = reply.send(backend.set(&id, feature, value));
                }
                Command::Probe(id, feature, reply) => {
                    let _ = reply.send(backend.probe(&id, feature));
                }
                Command::Capabilities(id, feature, reply) => {
                    let _ = reply.send(backend.capabilities(&id, feature));
                }
            }
        }
        // `backend` and its handles are dropped here, on their owning thread.
    }

    impl Drop for Session {
        fn drop(&mut self) {
            drop(self.sender.take());
            if let Some(worker) = self.worker.take() {
                // Includes backend destruction, even after initialization failure/panic.
                let _ = worker.join();
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::error::{DISPLAY_NOT_FOUND, INTERNAL_ERROR, TRANSPORT_ERROR};
        use std::cell::Cell;
        use std::rc::Rc;
        use std::sync::{Arc, Barrier, Mutex};
        use std::thread::ThreadId;

        #[derive(Debug, PartialEq)]
        enum Event {
            Create(ThreadId),
            Refresh(ThreadId),
            Read(ThreadId, String, Feature),
            Set(ThreadId, String, Feature, u16),
            Probe(ThreadId, String, Feature),
            Drop(ThreadId),
        }

        type Events = Arc<Mutex<Vec<Event>>>;

        struct Mock {
            events: Events,
            // This makes the backend !Send and !Sync, including its state.
            value: Rc<Cell<u16>>,
            fail_refresh: bool,
        }

        impl Mock {
            fn new(events: Events) -> Self {
                events
                    .lock()
                    .unwrap()
                    .push(Event::Create(thread::current().id()));
                Self {
                    events,
                    value: Rc::new(Cell::new(40)),
                    fail_refresh: false,
                }
            }

            fn record(&self, event: Event) {
                self.events.lock().unwrap().push(event);
            }

            fn check_id(&self, id: &str) -> Result<(), ApiError> {
                if id == "display" {
                    Ok(())
                } else {
                    Err(ApiError::new(DISPLAY_NOT_FOUND, "missing display"))
                }
            }

            fn value(&self) -> VcpValue {
                VcpValue {
                    current: self.value.get(),
                    maximum: 100,
                }
            }
        }

        impl Backend for Mock {
            fn refresh(&mut self) -> Result<Vec<DisplayInfo>, ApiError> {
                self.record(Event::Refresh(thread::current().id()));
                if self.fail_refresh {
                    return Err(ApiError::new(TRANSPORT_ERROR, "enumeration failed"));
                }
                Ok(vec![DisplayInfo::new(
                    "display".into(),
                    "mock".into(),
                    1,
                    2,
                    3,
                    "mock-location".into(),
                )])
            }

            fn read(&mut self, id: &str, feature: Feature) -> Result<VcpValue, ApiError> {
                self.record(Event::Read(thread::current().id(), id.into(), feature));
                assert_ne!(id, "panic", "mock operation panic");
                self.check_id(id)?;
                Ok(self.value())
            }

            fn set(
                &mut self,
                id: &str,
                feature: Feature,
                value: u16,
            ) -> Result<VcpValue, ApiError> {
                self.record(Event::Set(
                    thread::current().id(),
                    id.into(),
                    feature,
                    value,
                ));
                self.check_id(id)?;
                self.value.set(value);
                Ok(self.value())
            }

            fn probe(&mut self, id: &str, feature: Feature) -> Result<Option<VcpValue>, ApiError> {
                self.record(Event::Probe(thread::current().id(), id.into(), feature));
                self.check_id(id)?;
                Ok((feature == Feature::Brightness).then(|| self.value()))
            }

            fn capabilities(&mut self, id: &str, _feature: Feature) -> Result<Vec<u8>, ApiError> {
                self.check_id(id)?;
                Ok(vec![0, 2, 4])
            }
        }

        impl Drop for Mock {
            fn drop(&mut self) {
                self.record(Event::Drop(thread::current().id()));
            }
        }

        fn session(events: &Events) -> Session {
            let events = Arc::clone(events);
            Session::with_factory(move || Ok(Mock::new(events))).unwrap()
        }

        fn assert_lifecycle(events: &[Event]) {
            let Event::Create(owner) = events.first().unwrap() else {
                panic!("missing creation")
            };
            assert_ne!(*owner, thread::current().id());
            assert_eq!(events.last(), Some(&Event::Drop(*owner)));
            for event in events {
                let id = match event {
                    Event::Create(id)
                    | Event::Refresh(id)
                    | Event::Read(id, ..)
                    | Event::Set(id, ..)
                    | Event::Probe(id, ..)
                    | Event::Drop(id) => id,
                };
                assert_eq!(id, owner);
            }
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::Drop(_)))
                    .count(),
                1
            );
        }

        #[test]
        fn non_send_backend_creation_operations_and_drop_stay_on_worker() {
            let events = Events::default();
            let session = session(&events);
            assert_eq!(session.refresh().unwrap()[0].id(), "display");
            assert_eq!(
                session
                    .read("display".into(), Feature::Brightness)
                    .unwrap()
                    .current,
                40
            );
            assert_eq!(
                session
                    .set("display".into(), Feature::Brightness, 65)
                    .unwrap()
                    .current,
                65
            );
            assert_eq!(
                session
                    .read("display".into(), Feature::Brightness)
                    .unwrap()
                    .current,
                65
            );
            assert_eq!(
                session
                    .probe("display".into(), Feature::Brightness)
                    .unwrap()
                    .unwrap()
                    .current,
                65
            );
            assert_eq!(
                session.probe("display".into(), Feature::Gamma).unwrap(),
                None
            );
            assert_eq!(
                session
                    .read("missing".into(), Feature::Gamma)
                    .unwrap_err()
                    .status,
                DISPLAY_NOT_FOUND
            );
            // Ordinary errors leave the backend usable.
            assert!(session.refresh().is_ok());
            drop(session);
            let events = events.lock().unwrap();
            assert_lifecycle(&events);
            let owner = match events[0] {
                Event::Create(id) => id,
                _ => unreachable!(),
            };
            assert!(events.contains(&Event::Set(
                owner,
                "display".into(),
                Feature::Brightness,
                65
            )));
            assert!(events.contains(&Event::Probe(owner, "display".into(), Feature::Gamma)));
        }

        #[test]
        fn initialization_failure_is_returned_and_resources_are_joined() {
            let events = Events::default();
            let log = Arc::clone(&events);
            let result = Session::with_factory(move || {
                let _resource = Mock::new(log);
                Err::<Mock, _>(ApiError::new(TRANSPORT_ERROR, "initialization failed"))
            });
            let error = result.err().unwrap();
            assert_eq!(error.status, TRANSPORT_ERROR);
            assert_eq!(error.message, "initialization failed");
            assert_lifecycle(&events.lock().unwrap());
        }

        #[test]
        fn factory_panic_is_internal_and_joins_resources() {
            let events = Events::default();
            let log = Arc::clone(&events);
            let result = Session::with_factory(move || -> Result<Mock, ApiError> {
                let _resource = Mock::new(log);
                panic!("factory panic");
            });
            assert_eq!(result.err().unwrap().status, INTERNAL_ERROR);
            assert_lifecycle(&events.lock().unwrap());
        }

        #[test]
        fn operation_panic_stops_backend_instead_of_reusing_it() {
            let events = Events::default();
            let session = session(&events);
            assert_eq!(
                session
                    .read("panic".into(), Feature::Brightness)
                    .unwrap_err()
                    .status,
                INTERNAL_ERROR
            );
            assert_eq!(session.refresh().unwrap_err().status, INTERNAL_ERROR);
            assert_eq!(
                session
                    .set("display".into(), Feature::Gamma, 1)
                    .unwrap_err()
                    .status,
                INTERNAL_ERROR
            );
            assert_eq!(
                session
                    .probe("display".into(), Feature::Gamma)
                    .unwrap_err()
                    .status,
                INTERNAL_ERROR
            );
            drop(session);
            let events = events.lock().unwrap();
            assert_lifecycle(&events);
            assert_eq!(events.len(), 3, "create, panicking read, drop only");
        }

        #[test]
        fn refresh_error_does_not_kill_worker_or_discard_backend_state() {
            let events = Events::default();
            let log = Arc::clone(&events);
            let session = Session::with_factory(move || {
                let mut backend = Mock::new(log);
                backend.fail_refresh = true;
                Ok(backend)
            })
            .unwrap();
            session
                .set("display".into(), Feature::Brightness, 73)
                .unwrap();
            assert_eq!(session.refresh().unwrap_err().status, TRANSPORT_ERROR);
            assert_eq!(
                session
                    .read("display".into(), Feature::Brightness)
                    .unwrap()
                    .current,
                73
            );
            drop(session);
            assert_lifecycle(&events.lock().unwrap());
        }

        #[test]
        fn concurrent_callers_get_their_own_replies_and_share_one_worker() {
            let events = Events::default();
            let session = Arc::new(session(&events));
            let barrier = Arc::new(Barrier::new(33));
            let callers: Vec<_> = (0..32)
                .map(|value| {
                    let session = Arc::clone(&session);
                    let barrier = Arc::clone(&barrier);
                    thread::spawn(move || {
                        barrier.wait();
                        for _ in 0..4 {
                            let reply = session
                                .set("display".into(), Feature::Brightness, value)
                                .unwrap();
                            assert_eq!(
                                reply,
                                VcpValue {
                                    current: value,
                                    maximum: 100
                                }
                            );
                        }
                    })
                })
                .collect();
            barrier.wait();
            for caller in callers {
                caller.join().unwrap();
            }
            drop(session);
            let events = events.lock().unwrap();
            assert_lifecycle(&events);
            assert_eq!(events.len(), 130);
        }

        #[test]
        fn queued_calls_are_disconnected_after_panic() {
            let events = Events::default();
            let session = Arc::new(session(&events));
            let barrier = Arc::new(Barrier::new(33));
            let callers: Vec<_> = (0..32)
                .map(|_| {
                    let session = Arc::clone(&session);
                    let barrier = Arc::clone(&barrier);
                    thread::spawn(move || {
                        barrier.wait();
                        assert_eq!(
                            session
                                .read("panic".into(), Feature::Gamma)
                                .unwrap_err()
                                .status,
                            INTERNAL_ERROR
                        );
                    })
                })
                .collect();
            barrier.wait();
            for caller in callers {
                caller.join().unwrap();
            }
            drop(session);
            let events = events.lock().unwrap();
            assert_lifecycle(&events);
            assert_eq!(events.len(), 3);
        }

        #[test]
        fn dropping_idle_session_joins_backend_destruction() {
            let events = Events::default();
            drop(session(&events));
            let events = events.lock().unwrap();
            assert_lifecycle(&events);
            assert_eq!(events.len(), 2);
        }

        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        #[test]
        fn public_constructor_starts_native_worker_without_enumerating() {
            let session = Session::new().unwrap();
            assert_eq!(
                session
                    .read("missing".into(), Feature::Brightness)
                    .unwrap_err()
                    .status,
                crate::error::DISPLAY_NOT_FOUND
            );
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        #[test]
        fn public_constructor_rejects_unsupported_platform() {
            assert_eq!(
                Session::new().err().unwrap().status,
                crate::error::UNSUPPORTED_PLATFORM
            );
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows", test)))]
mod implementation {
    use crate::error::{ApiError, UNSUPPORTED_PLATFORM};
    use agon_osd::{DisplayInfo, Feature, VcpValue};

    pub(crate) struct Session;

    fn unsupported() -> ApiError {
        ApiError::new(
            UNSUPPORTED_PLATFORM,
            "display control supports macOS, Linux and Windows",
        )
    }

    impl Session {
        pub(crate) fn new() -> Result<Self, ApiError> {
            Err(unsupported())
        }
        pub(crate) fn refresh(&self) -> Result<Vec<DisplayInfo>, ApiError> {
            Err(unsupported())
        }
        pub(crate) fn read(&self, _id: String, _feature: Feature) -> Result<VcpValue, ApiError> {
            Err(unsupported())
        }
        pub(crate) fn set(
            &self,
            _id: String,
            _feature: Feature,
            _value: u16,
        ) -> Result<VcpValue, ApiError> {
            Err(unsupported())
        }
        pub(crate) fn probe(
            &self,
            _id: String,
            _feature: Feature,
        ) -> Result<Option<VcpValue>, ApiError> {
            Err(unsupported())
        }
        pub(crate) fn capabilities(
            &self,
            _id: String,
            _feature: Feature,
        ) -> Result<Vec<u8>, ApiError> {
            Err(unsupported())
        }
    }
}

pub(crate) use implementation::Session;
