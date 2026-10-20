//! Background worker that owns the mouse connection and applies profile rules on focus changes.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use serde::Serialize;

use crate::config::Config;
use crate::mouse::{self, Mouse, Notification};

/// Focus must settle this long before switching, so alt-tabbing through apps doesn't spam writes.
const DEBOUNCE: Duration = Duration::from_millis(300);
const RECONNECT_EVERY: Duration = Duration::from_secs(2);
/// When the mouse is asleep, retry a pending switch this often until it answers.
const RETRY_EVERY: Duration = Duration::from_secs(3);
const POLL: Duration = Duration::from_millis(100);
const RECENT_APPS: usize = 20;

pub enum Msg {
    Focus(String),
    Config(Config),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Status {
    /// A receiver or cable exposing the Keychron collection is open.
    pub connected: bool,
    /// `None` until the mouse has answered (it may be asleep).
    pub profile: Option<u8>,
    pub profile_count: Option<u8>,
    pub current_app: Option<String>,
    /// Most recent first, for picking apps in the settings window.
    pub recent_apps: Vec<String>,
}

pub type StatusHandle = Arc<Mutex<Status>>;

pub fn spawn(
    config: Config,
    on_change: impl Fn(&Status) + Send + 'static,
) -> (Sender<Msg>, StatusHandle) {
    let (tx, rx) = mpsc::channel();
    let status = StatusHandle::default();
    let worker = Worker {
        rx,
        config,
        status: status.clone(),
        on_change: Box::new(on_change),
        mouse: None,
        next_connect: Instant::now(),
        pending_focus: None,
        retry: None,
    };
    thread::Builder::new()
        .name("switcher".into())
        .spawn(move || worker.run())
        .expect("spawn switcher thread");
    (tx, status)
}

struct Worker {
    rx: Receiver<Msg>,
    config: Config,
    status: StatusHandle,
    on_change: Box<dyn Fn(&Status) + Send>,
    mouse: Option<Mouse>,
    next_connect: Instant,
    /// App to evaluate once the debounce deadline passes.
    pending_focus: Option<(String, Instant)>,
    /// Profile we still owe the mouse, and when to try again.
    retry: Option<(u8, Instant)>,
}

impl Worker {
    fn run(mut self) {
        let mut api = match HidApi::new() {
            Ok(api) => api,
            Err(e) => return eprintln!("hidapi init failed: {e}"),
        };
        loop {
            if self.mouse.is_none() && Instant::now() >= self.next_connect {
                self.connect(&mut api);
            }

            match self.rx.recv_timeout(self.wait_time()) {
                Ok(Msg::Focus(app)) => self.on_focus(app),
                Ok(Msg::Config(config)) => {
                    self.config = config;
                    // Re-apply rules to the app that's in front right now.
                    if let Some(app) = self.status().current_app {
                        self.pending_focus = Some((app, Instant::now()));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }

            self.drain_notifications();

            let now = Instant::now();
            if let Some((app, _)) = self.pending_focus.take_if(|(_, due)| *due <= now) {
                if let Some(target) = self.config.profile_for(&app) {
                    self.switch_to(target);
                }
            }
            if let Some((target, _)) = self.retry.take_if(|(_, due)| *due <= now) {
                self.switch_to(target);
            }
        }
    }

    fn wait_time(&self) -> Duration {
        let now = Instant::now();
        [
            self.pending_focus.as_ref().map(|p| p.1),
            self.retry.map(|r| r.1),
        ]
        .into_iter()
        .flatten()
        .map(|due| due.saturating_duration_since(now))
        .fold(POLL, Duration::min)
    }

    fn connect(&mut self, api: &mut HidApi) {
        self.next_connect = Instant::now() + RECONNECT_EVERY;
        if api.refresh_devices().is_err() {
            return;
        }
        let Some(mouse) = Mouse::open(api) else {
            return;
        };
        eprintln!("mouse connected");
        self.mouse = Some(mouse);
        self.update(|s| s.connected = true);
        self.refresh_info();
        // Apply the rule for whatever is focused now.
        if let Some(app) = self.status().current_app {
            self.pending_focus = Some((app, Instant::now()));
        }
    }

    fn disconnect(&mut self, err: &mouse::Error) {
        eprintln!("mouse disconnected: {err}");
        self.mouse = None;
        self.next_connect = Instant::now() + RECONNECT_EVERY;
        self.update(|s| {
            s.connected = false;
            s.profile = None;
            s.profile_count = None;
        });
    }

    fn refresh_info(&mut self) {
        let Some(mouse) = self.mouse.as_mut() else {
            return;
        };
        match mouse.base_info() {
            Ok(info) => {
                eprintln!(
                    "base info: profile {} of {}",
                    info.profile + 1,
                    info.profile_count
                );
                self.update(|s| {
                    s.profile = Some(info.profile);
                    s.profile_count = Some(info.profile_count);
                })
            }
            Err(mouse::Error::Timeout) => {} // asleep; we'll ask again when it wakes
            Err(e) => self.disconnect(&e),
        }
    }

    fn on_focus(&mut self, app: String) {
        eprintln!("focus: {app}");
        self.update(|s| {
            s.recent_apps.retain(|a| *a != app);
            s.recent_apps.insert(0, app.clone());
            s.recent_apps.truncate(RECENT_APPS);
            s.current_app = Some(app.clone());
        });
        self.pending_focus = Some((app, Instant::now() + DEBOUNCE));
        // A newer focus change supersedes any switch we were still retrying.
        self.retry = None;
    }

    fn drain_notifications(&mut self) {
        while let Some(mouse) = self.mouse.as_mut() {
            match mouse.poll(Duration::ZERO) {
                Ok(None) => break,
                Ok(Some(Notification::Profile(p))) => {
                    // Changed on the mouse itself: respect it until the next focus change.
                    self.retry = None;
                    self.update(|s| s.profile = Some(p));
                }
                Ok(Some(Notification::Status { connected: true })) => {
                    if self.status().profile.is_none() {
                        self.refresh_info();
                    }
                    if let Some((_, due)) = self.retry.as_mut() {
                        *due = Instant::now();
                    }
                }
                Ok(Some(Notification::Status { connected: false })) => {}
                Err(e) => self.disconnect(&e),
            }
        }
    }

    fn switch_to(&mut self, target: u8) {
        if self.mouse.is_none() {
            return; // connect() re-evaluates the focused app
        }
        if self.status().profile_count.is_none() {
            self.refresh_info();
        }
        let status = self.status();
        let Some(count) = status.profile_count else {
            self.retry = Some((target, Instant::now() + RETRY_EVERY));
            return;
        };
        if target >= count || status.profile == Some(target) {
            return;
        }
        let Some(mouse) = self.mouse.as_mut() else {
            return;
        };
        match mouse.set_profile(target, count) {
            Ok(()) => {
                eprintln!("switched to profile {}", target + 1);
                self.update(|s| s.profile = Some(target))
            }
            Err(mouse::Error::Timeout) => {
                eprintln!("profile {} not acknowledged, will retry", target + 1);
                self.retry = Some((target, Instant::now() + RETRY_EVERY))
            }
            Err(e) => self.disconnect(&e),
        }
    }

    fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    fn update(&self, f: impl FnOnce(&mut Status)) {
        let snapshot = {
            let mut s = self.status.lock().unwrap();
            f(&mut s);
            s.clone()
        };
        (self.on_change)(&snapshot);
    }
}
