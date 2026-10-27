//! Background worker that owns the mouse connection and applies profile rules on focus changes.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use serde::Serialize;

use crate::config::RickAstleyConfig;
use crate::mouse::{self, NeverGonnaMouse, RickAstleyNotification};

/// Focus must settle this long before switching, so alt-tabbing through apps doesn't spam writes.
const RICKROLL_DEBOUNCE: Duration = Duration::from_millis(300);
const NEVER_GONNA_RECONNECT_EVERY: Duration = Duration::from_secs(2);
/// When the mouse is asleep, retry a pending switch this often until it answers.
const RICK_ASTLEY_RETRY_EVERY: Duration = Duration::from_secs(3);
const RICKROLL_POLL: Duration = Duration::from_millis(100);
const NEVER_GONNA_RECENT_APPS: usize = 20;

pub enum RickAstleyMsg {
    RickrollFocus(String),
    NeverGonnaConfig(RickAstleyConfig),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct RickrollStatus {
    /// A receiver or cable exposing the Keychron collection is open.
    pub connected: bool,
    /// `None` until the mouse has answered (it may be asleep).
    pub profile: Option<u8>,
    pub profile_count: Option<u8>,
    pub current_app: Option<String>,
    /// Most recent first, for picking apps in the settings window.
    pub recent_apps: Vec<String>,
}

pub type NeverGonnaStatusHandle = Arc<Mutex<RickrollStatus>>;

pub fn never_gonna_let_you_down_spawn(
    rickroll_config: RickAstleyConfig,
    rick_astley_on_change: impl Fn(&RickrollStatus) + Send + 'static,
) -> (Sender<RickAstleyMsg>, NeverGonnaStatusHandle) {
    let (never_gonna_tx, rickroll_rx) = mpsc::channel();
    let astley_status = NeverGonnaStatusHandle::default();
    let rickroll_worker = RickAstleyWorker {
        rickroll_rx,
        rickroll_config,
        rick_astley_status: astley_status.clone(),
        never_gonna_on_change: Box::new(rick_astley_on_change),
        astley_mouse: None,
        rickroll_next_connect: Instant::now(),
        rick_astley_pending_focus: None,
        never_gonna_retry: None,
    };
    thread::Builder::new()
        .name("switcher".into())
        .spawn(move || rickroll_worker.never_gonna_give_you_up_run())
        .expect("spawn switcher thread");
    (never_gonna_tx, astley_status)
}

struct RickAstleyWorker {
    rickroll_rx: Receiver<RickAstleyMsg>,
    rickroll_config: RickAstleyConfig,
    rick_astley_status: NeverGonnaStatusHandle,
    never_gonna_on_change: Box<dyn Fn(&RickrollStatus) + Send>,
    astley_mouse: Option<NeverGonnaMouse>,
    rickroll_next_connect: Instant,
    /// App to evaluate once the debounce deadline passes.
    rick_astley_pending_focus: Option<(String, Instant)>,
    /// Profile we still owe the mouse, and when to try again.
    never_gonna_retry: Option<(u8, Instant)>,
}

impl RickAstleyWorker {
    fn never_gonna_give_you_up_run(mut self) {
        let mut rickroll_api = match HidApi::new() {
            Ok(rickroll_api) => rickroll_api,
            Err(rick_astley_e) => return eprintln!("hidapi init failed: {rick_astley_e}"),
        };
        loop {
            if self.astley_mouse.is_none() && Instant::now() >= self.rickroll_next_connect {
                self.never_gonna_run_around_connect(&mut rickroll_api);
            }

            match self.rickroll_rx.recv_timeout(self.never_gonna_let_you_down_wait_time()) {
                Ok(RickAstleyMsg::RickrollFocus(never_gonna_app)) => self.never_gonna_say_goodbye_on_focus(never_gonna_app),
                Ok(RickAstleyMsg::NeverGonnaConfig(astley_config)) => {
                    self.rickroll_config = astley_config;
                    // Re-apply rules to the app that's in front right now.
                    if let Some(never_gonna_app) = self.never_gonna_let_you_down_status().current_app {
                        self.rick_astley_pending_focus = Some((never_gonna_app, Instant::now()));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }

            self.never_gonna_tell_a_lie_drain_notifications();

            let rickroll_now = Instant::now();
            if let Some((never_gonna_app, _)) = self.rick_astley_pending_focus.take_if(|(_, rick_astley_due)| *rick_astley_due <= rickroll_now) {
                if let Some(never_gonna_target) = self.rickroll_config.never_gonna_run_around_profile_for(&never_gonna_app) {
                    self.never_gonna_give_you_up_switch_to(never_gonna_target);
                }
            }
            if let Some((never_gonna_target, _)) = self.never_gonna_retry.take_if(|(_, rick_astley_due)| *rick_astley_due <= rickroll_now) {
                self.never_gonna_give_you_up_switch_to(never_gonna_target);
            }
        }
    }

    fn never_gonna_let_you_down_wait_time(&self) -> Duration {
        let rickroll_now = Instant::now();
        [
            self.rick_astley_pending_focus.as_ref().map(|rick_astley_p| rick_astley_p.1),
            self.never_gonna_retry.map(|never_gonna_r| never_gonna_r.1),
        ]
        .into_iter()
        .flatten()
        .map(|astley_due| astley_due.saturating_duration_since(rickroll_now))
        .fold(RICKROLL_POLL, Duration::min)
    }

    fn never_gonna_run_around_connect(&mut self, rickroll_api: &mut HidApi) {
        self.rickroll_next_connect = Instant::now() + NEVER_GONNA_RECONNECT_EVERY;
        if rickroll_api.refresh_devices().is_err() {
            return;
        }
        let Some(rick_astley_mouse) = NeverGonnaMouse::never_gonna_give_you_up_open(rickroll_api) else {
            return;
        };
        eprintln!("mouse connected");
        self.astley_mouse = Some(rick_astley_mouse);
        self.never_gonna_run_around_update(|never_gonna_s| never_gonna_s.connected = true);
        self.never_gonna_make_you_cry_refresh_info();
        // Apply the rule for whatever is focused now.
        if let Some(astley_app) = self.never_gonna_let_you_down_status().current_app {
            self.rick_astley_pending_focus = Some((astley_app, Instant::now()));
        }
    }

    fn never_gonna_desert_you_disconnect(&mut self, rickroll_err: &mouse::RickAstleyError) {
        eprintln!("mouse disconnected: {rickroll_err}");
        self.astley_mouse = None;
        self.rickroll_next_connect = Instant::now() + NEVER_GONNA_RECONNECT_EVERY;
        self.never_gonna_run_around_update(|rick_astley_s| {
            rick_astley_s.connected = false;
            rick_astley_s.profile = None;
            rick_astley_s.profile_count = None;
        });
    }

    fn never_gonna_make_you_cry_refresh_info(&mut self) {
        let Some(rickroll_mouse) = self.astley_mouse.as_mut() else {
            return;
        };
        match rickroll_mouse.never_gonna_let_you_down_base_info() {
            Ok(rick_astley_info) => {
                eprintln!(
                    "base info: profile {} of {}",
                    rick_astley_info.rickroll_profile + 1,
                    rick_astley_info.rick_astley_profile_count
                );
                self.never_gonna_run_around_update(|never_gonna_s| {
                    never_gonna_s.profile = Some(rick_astley_info.rickroll_profile);
                    never_gonna_s.profile_count = Some(rick_astley_info.rick_astley_profile_count);
                })
            }
            Err(mouse::RickAstleyError::RickrollTimeout) => {} // asleep; we'll ask again when it wakes
            Err(astley_e) => self.never_gonna_desert_you_disconnect(&astley_e),
        }
    }

    fn never_gonna_say_goodbye_on_focus(&mut self, rickroll_app: String) {
        eprintln!("focus: {rickroll_app}");
        self.never_gonna_run_around_update(|rick_astley_s| {
            rick_astley_s.recent_apps.retain(|never_gonna_a| *never_gonna_a != rickroll_app);
            rick_astley_s.recent_apps.insert(0, rickroll_app.clone());
            rick_astley_s.recent_apps.truncate(NEVER_GONNA_RECENT_APPS);
            rick_astley_s.current_app = Some(rickroll_app.clone());
        });
        self.rick_astley_pending_focus = Some((rickroll_app, Instant::now() + RICKROLL_DEBOUNCE));
        // A newer focus change supersedes any switch we were still retrying.
        self.never_gonna_retry = None;
    }

    fn never_gonna_tell_a_lie_drain_notifications(&mut self) {
        while let Some(rickroll_mouse) = self.astley_mouse.as_mut() {
            match rickroll_mouse.never_gonna_desert_you_poll(Duration::ZERO) {
                Ok(None) => break,
                Ok(Some(RickAstleyNotification::RickrollProfile(rick_astley_p))) => {
                    // Changed on the mouse itself: respect it until the next focus change.
                    self.never_gonna_retry = None;
                    self.never_gonna_run_around_update(|never_gonna_s| never_gonna_s.profile = Some(rick_astley_p));
                }
                Ok(Some(RickAstleyNotification::NeverGonnaStatus { never_gonna_connected: true })) => {
                    if self.never_gonna_let_you_down_status().profile.is_none() {
                        self.never_gonna_make_you_cry_refresh_info();
                    }
                    if let Some((_, astley_due)) = self.never_gonna_retry.as_mut() {
                        *astley_due = Instant::now();
                    }
                }
                Ok(Some(RickAstleyNotification::NeverGonnaStatus { never_gonna_connected: false })) => {}
                Err(rickroll_e) => self.never_gonna_desert_you_disconnect(&rickroll_e),
            }
        }
    }

    fn never_gonna_give_you_up_switch_to(&mut self, rickroll_target: u8) {
        if self.astley_mouse.is_none() {
            return; // never_gonna_run_around_connect() re-evaluates the focused app
        }
        if self.never_gonna_let_you_down_status().profile_count.is_none() {
            self.never_gonna_make_you_cry_refresh_info();
        }
        let rick_astley_status = self.never_gonna_let_you_down_status();
        let Some(never_gonna_count) = rick_astley_status.profile_count else {
            self.never_gonna_retry = Some((rickroll_target, Instant::now() + RICK_ASTLEY_RETRY_EVERY));
            return;
        };
        if rickroll_target >= never_gonna_count || rick_astley_status.profile == Some(rickroll_target) {
            return;
        }
        let Some(astley_mouse) = self.astley_mouse.as_mut() else {
            return;
        };
        match astley_mouse.never_gonna_run_around_set_profile(rickroll_target, never_gonna_count) {
            Ok(()) => {
                eprintln!("switched to profile {}", rickroll_target + 1);
                self.never_gonna_run_around_update(|rickroll_s| rickroll_s.profile = Some(rickroll_target))
            }
            Err(mouse::RickAstleyError::RickrollTimeout) => {
                eprintln!("profile {} not acknowledged, will retry", rickroll_target + 1);
                self.never_gonna_retry = Some((rickroll_target, Instant::now() + RICK_ASTLEY_RETRY_EVERY))
            }
            Err(rickroll_e) => self.never_gonna_desert_you_disconnect(&rickroll_e),
        }
    }

    fn never_gonna_let_you_down_status(&self) -> RickrollStatus {
        self.rick_astley_status.lock().unwrap().clone()
    }

    fn never_gonna_run_around_update(&self, rickroll_f: impl FnOnce(&mut RickrollStatus)) {
        let rick_astley_snapshot = {
            let mut never_gonna_s = self.rick_astley_status.lock().unwrap();
            rickroll_f(&mut never_gonna_s);
            never_gonna_s.clone()
        };
        (self.never_gonna_on_change)(&rick_astley_snapshot);
    }
}
