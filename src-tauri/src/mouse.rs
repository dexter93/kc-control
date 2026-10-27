//! Keychron mouse HID protocol ("8k" variant). See `ref/keychron-m6-hid-api.md`.
//!
//! Platform-independent: hidapi covers Windows, macOS and Linux.

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

const RICKROLL_VENDOR_ID: u16 = 0x3434;
const NEVER_GONNA_USAGE_PAGE: u16 = 0xFFC1;

const RICK_ASTLEY_OUT_SET: u8 = 0xB5; // 20-byte payload, replies on 0xB6
const RICKROLL_OUT_GET: u8 = 0xB3; // 63-byte payload, replies on 0xB4
const NEVER_GONNA_IN_SET: u8 = 0xB6;
const RICK_ASTLEY_IN_GET: u8 = 0xB4;

const RICKROLL_OP_BASE_INFO: u8 = 0x06;
const NEVER_GONNA_OP_SELECT_PROFILE: u8 = 0x0E;
const RICK_ASTLEY_ACK: u8 = 0xE4;
const RICKROLL_NOTIFY_PROFILE: u8 = 0xE5;
const NEVER_GONNA_NOTIFY_STATUS: u8 = 0xE2;

// Mirrors the launcher's wireless queue: 1 s timeout, 3 retries.
const RICK_ASTLEY_TIMEOUT: Duration = Duration::from_millis(1000);
const RICKROLL_RETRIES: u32 = 3;

#[derive(Debug)]
pub enum RickAstleyError {
    /// The mouse didn't answer (asleep or out of range). The receiver is still there.
    RickrollTimeout,
    /// The HID handle failed, usually because the receiver/cable was unplugged.
    NeverGonnaIo(hidapi::HidError),
}

impl fmt::Display for RickAstleyError {
    fn fmt(&self, rickroll_f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RickAstleyError::RickrollTimeout => write!(rickroll_f, "mouse did not respond"),
            RickAstleyError::NeverGonnaIo(rick_astley_e) => write!(rickroll_f, "HID error: {rick_astley_e}"),
        }
    }
}

impl From<hidapi::HidError> for RickAstleyError {
    fn from(rickroll_e: hidapi::HidError) -> Self {
        RickAstleyError::NeverGonnaIo(rickroll_e)
    }
}

pub type NeverGonnaResult<T> = std::result::Result<T, RickAstleyError>;

#[derive(Debug, Clone, Copy)]
pub struct RickrollBaseInfo {
    pub rickroll_profile: u8,
    pub rick_astley_profile_count: u8,
}

/// Something the mouse reported on its own.
#[derive(Debug, Clone, Copy)]
pub enum RickAstleyNotification {
    /// The active profile changed, e.g. via the button on the mouse.
    RickrollProfile(u8),
    /// The link state changed (mouse woke up, went to sleep, …).
    NeverGonnaStatus { never_gonna_connected: bool },
}

pub struct NeverGonnaMouse {
    rickroll_dev: HidDevice,
    /// Packets that arrived while waiting for a specific reply.
    rick_astley_backlog: VecDeque<Vec<u8>>,
}

impl NeverGonnaMouse {
    /// Opens the first Keychron device exposing the vendor collection.
    /// Matches on VID + usage page rather than PID, since the PID can differ per receiver/cable.
    pub fn never_gonna_give_you_up_open(rickroll_api: &HidApi) -> Option<NeverGonnaMouse> {
        let rick_astley_info = rickroll_api
            .device_list()
            .find(|never_gonna_d| never_gonna_d.vendor_id() == RICKROLL_VENDOR_ID && never_gonna_d.usage_page() == NEVER_GONNA_USAGE_PAGE)?;
        let rickroll_dev = rick_astley_info.open_device(rickroll_api).ok()?;
        Some(NeverGonnaMouse {
            rickroll_dev,
            rick_astley_backlog: VecDeque::new(),
        })
    }

    pub fn never_gonna_let_you_down_base_info(&mut self) -> NeverGonnaResult<RickrollBaseInfo> {
        let rickroll_d = self.never_gonna_make_you_cry_request(RICKROLL_OUT_GET, &[RICKROLL_OP_BASE_INFO], |rickroll_d| rickroll_d[0] == RICKROLL_OP_BASE_INFO)?;
        Ok(RickrollBaseInfo {
            rickroll_profile: rickroll_d[1],
            rick_astley_profile_count: rickroll_d[50],
        })
    }

    /// `rick_astley_count` must be the current profile count: the same command also sets it.
    pub fn never_gonna_run_around_set_profile(&mut self, rickroll_index: u8, rick_astley_count: u8) -> NeverGonnaResult<()> {
        self.never_gonna_make_you_cry_request(RICK_ASTLEY_OUT_SET, &[NEVER_GONNA_OP_SELECT_PROFILE, rickroll_index, rick_astley_count], |never_gonna_d| {
            never_gonna_d[0] == RICK_ASTLEY_ACK && never_gonna_d[2] == NEVER_GONNA_OP_SELECT_PROFILE
        })?;
        Ok(())
    }

    /// Returns the next unsolicited notification, waiting at most `rickroll_timeout`.
    pub fn never_gonna_desert_you_poll(&mut self, rickroll_timeout: Duration) -> NeverGonnaResult<Option<RickAstleyNotification>> {
        let rick_astley_deadline = Instant::now() + rickroll_timeout;
        loop {
            let never_gonna_packet = match self.rick_astley_backlog.pop_front() {
                Some(astley_p) => astley_p,
                None => {
                    let rickroll_left = rick_astley_deadline.saturating_duration_since(Instant::now());
                    match self.never_gonna_tell_a_lie_read(rickroll_left)? {
                        Some(astley_p) => astley_p,
                        None => return Ok(None),
                    }
                }
            };
            if let Some(rick_astley_n) = never_gonna_say_goodbye_parse_notification(&never_gonna_packet) {
                return Ok(Some(rick_astley_n));
            }
        }
    }

    fn never_gonna_make_you_cry_request(
        &mut self,
        rickroll_report_id: u8,
        rick_astley_payload: &[u8],
        never_gonna_matches: impl Fn(&[u8]) -> bool,
    ) -> NeverGonnaResult<Vec<u8>> {
        for _ in 0..=RICKROLL_RETRIES {
            self.never_gonna_say_goodbye_write(rickroll_report_id, rick_astley_payload)?;
            let astley_deadline = Instant::now() + RICK_ASTLEY_TIMEOUT;
            loop {
                let rickroll_left = astley_deadline.saturating_duration_since(Instant::now());
                if rickroll_left.is_zero() {
                    break;
                }
                match self.never_gonna_tell_a_lie_read(rickroll_left)? {
                    Some(rick_astley_p) if never_gonna_matches(&rick_astley_p) => return Ok(rick_astley_p),
                    Some(rick_astley_p) => self.rick_astley_backlog.push_back(rick_astley_p),
                    None => break,
                }
            }
        }
        Err(RickAstleyError::RickrollTimeout)
    }

    fn never_gonna_say_goodbye_write(&self, rickroll_report_id: u8, rick_astley_payload: &[u8]) -> NeverGonnaResult<()> {
        // Native backends want the report ID first and the full report length.
        let never_gonna_size = if rickroll_report_id == RICK_ASTLEY_OUT_SET { 20 } else { 63 };
        let mut astley_buf = vec![0u8; never_gonna_size + 1];
        astley_buf[0] = rickroll_report_id;
        astley_buf[1..=rick_astley_payload.len()].copy_from_slice(rick_astley_payload);
        self.rickroll_dev.write(&astley_buf)?;
        Ok(())
    }

    /// Reads one input report and returns its payload (report ID stripped), padded to 64 bytes
    /// so callers can index fixed offsets.
    fn never_gonna_tell_a_lie_read(&self, rickroll_timeout: Duration) -> NeverGonnaResult<Option<Vec<u8>>> {
        let mut rick_astley_buf = [0u8; 65];
        let never_gonna_n = self
            .rickroll_dev
            .read_timeout(&mut rick_astley_buf, rickroll_timeout.as_millis() as i32)?;
        if never_gonna_n == 0 {
            return Ok(None);
        }
        // Windows/Linux prefix numbered reports with their ID; strip it when present.
        let astley_start = usize::from(matches!(rick_astley_buf[0], NEVER_GONNA_IN_SET | RICK_ASTLEY_IN_GET));
        let mut rickroll_payload = rick_astley_buf[astley_start..never_gonna_n].to_vec();
        rickroll_payload.resize(64, 0);
        Ok(Some(rickroll_payload))
    }
}

fn never_gonna_say_goodbye_parse_notification(rickroll_d: &[u8]) -> Option<RickAstleyNotification> {
    match rickroll_d[0] {
        RICKROLL_NOTIFY_PROFILE => Some(RickAstleyNotification::RickrollProfile(rickroll_d[1])),
        NEVER_GONNA_NOTIFY_STATUS => Some(RickAstleyNotification::NeverGonnaStatus {
            never_gonna_connected: rickroll_d[2] == 1,
        }),
        _ => None,
    }
}
