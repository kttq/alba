use std::{
    fmt,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use crate::config::Coordinates;

/// The app calls `finished` on every tick, so neither method may block.
pub trait WeatherSource {
    /// Does nothing while a fetch is running.
    fn request(&mut self, coordinates: Coordinates);

    /// The response body of the fetch that just finished, returned once.
    fn finished(&mut self) -> Option<Result<String, FetchError>>;
}

/// The text is for display only.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// DNS, connection, TLS, timeout, or a failed body read.
    Network(String),
    Status(u16),
    Invalid(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Network(reason) => write!(formatter, "network error: {reason}"),
            Self::Status(status) => write!(formatter, "http status {status}"),
            Self::Invalid(reason) => write!(formatter, "invalid forecast: {reason}"),
        }
    }
}

type Fetch = dyn Fn(Coordinates) -> Result<String, FetchError> + Send + Sync;

pub struct BackgroundSource {
    fetch: Arc<Fetch>,
    running: Option<Receiver<Result<String, FetchError>>>,
}

impl BackgroundSource {
    /// `fetch` must time out on its own.
    pub fn new(
        fetch: impl Fn(Coordinates) -> Result<String, FetchError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            fetch: Arc::new(fetch),
            running: None,
        }
    }
}

impl WeatherSource for BackgroundSource {
    fn request(&mut self, coordinates: Coordinates) {
        if self.running.is_some() {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let fetch = Arc::clone(&self.fetch);
        thread::spawn(move || {
            let _ = sender.send(fetch(coordinates));
        });
        self.running = Some(receiver);
    }

    fn finished(&mut self) -> Option<Result<String, FetchError>> {
        let result = match self.running.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(FetchError::Network(
                "the fetch ended without a result".to_string(),
            )),
        };
        self.running = None;
        Some(result)
    }
}
