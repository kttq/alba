use std::{
    fmt,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use crate::config::Coordinates;

/// Where the app gets forecasts. The app calls `finished` on every tick, so neither method may
/// block.
pub trait WeatherSource {
    /// Starts fetching the forecast for `coordinates`, unless a fetch is running.
    fn request(&mut self, coordinates: Coordinates);

    /// The response body of the fetch that just finished, returned once.
    fn finished(&mut self) -> Option<Result<String, FetchError>>;
}

/// Why a fetch gave no usable forecast. The text is for display only.
#[derive(Debug, Clone, PartialEq)]
pub enum FetchError {
    /// No response arrived: DNS, connection, TLS, or timeout.
    Network(String),
    /// The server answered with an HTTP error status.
    Status(u16),
    /// The response is not a complete forecast.
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

/// Runs each fetch on its own thread and sends the result back over a channel.
pub struct BackgroundSource {
    fetch: Arc<Fetch>,
    running: Option<Receiver<Result<String, FetchError>>>,
}

impl BackgroundSource {
    /// `fetch` blocks until it has a result, and must time out on its own.
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
            // The app may be gone by now; then nobody needs the result.
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
