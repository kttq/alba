//! The real weather source, over a fetch that never touches the network.

use std::{
    sync::{
        Mutex,
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

use alba::{
    config::Coordinates,
    weather::{BackgroundSource, FetchError, WeatherSource},
};

const BUENOS_AIRES: Coordinates = Coordinates {
    latitude: -34.6037,
    longitude: -58.3816,
};

/// A source whose fetches report their start, then wait for the test to release them.
fn gated_source() -> (BackgroundSource, Receiver<Coordinates>, mpsc::Sender<()>) {
    let (started, starts) = mpsc::channel();
    let (release, gate) = mpsc::channel::<()>();
    let started = Mutex::new(started);
    let gate = Mutex::new(gate);
    let source = BackgroundSource::new(move |coordinates| {
        started.lock().unwrap().send(coordinates).unwrap();
        gate.lock().unwrap().recv().unwrap();
        Ok("forecast".to_string())
    });
    (source, starts, release)
}

/// Polls like the app's tick does, without blocking on the source.
fn wait_until_finished(source: &mut BackgroundSource) -> Result<String, FetchError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = source.finished() {
            return result;
        }
        assert!(Instant::now() < deadline, "the fetch never finished");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn fetch_runs_in_the_background_and_returns_its_body_once() {
    let (mut source, starts, release) = gated_source();

    source.request(BUENOS_AIRES);

    assert_eq!(
        starts.recv_timeout(Duration::from_secs(5)),
        Ok(BUENOS_AIRES)
    );
    assert_eq!(source.finished(), None);
    release.send(()).unwrap();
    assert_eq!(wait_until_finished(&mut source), Ok("forecast".to_string()));
    assert_eq!(source.finished(), None);
}

#[test]
fn request_during_a_fetch_starts_no_second_fetch() {
    let (mut source, starts, release) = gated_source();

    source.request(BUENOS_AIRES);
    starts.recv_timeout(Duration::from_secs(5)).unwrap();
    source.request(BUENOS_AIRES);

    assert!(starts.recv_timeout(Duration::from_millis(100)).is_err());
    release.send(()).unwrap();
    wait_until_finished(&mut source).unwrap();
    source.request(BUENOS_AIRES);
    assert!(starts.recv_timeout(Duration::from_secs(5)).is_ok());
    release.send(()).unwrap();
}
