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
    weather::{BackgroundSource, FetchError, Request, WeatherSource},
};

const BUENOS_AIRES: Request = Request::Forecast(Coordinates {
    latitude: -34.6037,
    longitude: -58.3816,
});

fn gated_source() -> (BackgroundSource, Receiver<Request>, mpsc::Sender<()>) {
    let (started, starts) = mpsc::channel();
    let (release, gate) = mpsc::channel::<()>();
    let started = Mutex::new(started);
    let gate = Mutex::new(gate);
    let source = BackgroundSource::new(move |request| {
        started.lock().unwrap().send(request).unwrap();
        gate.lock().unwrap().recv().unwrap();
        Ok("forecast".to_string())
    });
    (source, starts, release)
}

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
