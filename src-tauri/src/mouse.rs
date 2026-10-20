//! Keychron mouse HID protocol ("8k" variant). See `ref/keychron-m6-hid-api.md`.
//!
//! Platform-independent: hidapi covers Windows, macOS and Linux.

use std::collections::VecDeque;
use std::fmt;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

const VENDOR_ID: u16 = 0x3434;
const USAGE_PAGE: u16 = 0xFFC1;

const OUT_SET: u8 = 0xB5; // 20-byte payload, replies on 0xB6
const OUT_GET: u8 = 0xB3; // 63-byte payload, replies on 0xB4
const IN_SET: u8 = 0xB6;
const IN_GET: u8 = 0xB4;

const OP_BASE_INFO: u8 = 0x06;
const OP_SELECT_PROFILE: u8 = 0x0E;
const ACK: u8 = 0xE4;
const NOTIFY_PROFILE: u8 = 0xE5;
const NOTIFY_STATUS: u8 = 0xE2;

// Mirrors the launcher's wireless queue: 1 s timeout, 3 retries.
const TIMEOUT: Duration = Duration::from_millis(1000);
const RETRIES: u32 = 3;

#[derive(Debug)]
pub enum Error {
    /// The mouse didn't answer (asleep or out of range). The receiver is still there.
    Timeout,
    /// The HID handle failed, usually because the receiver/cable was unplugged.
    Io(hidapi::HidError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Timeout => write!(f, "mouse did not respond"),
            Error::Io(e) => write!(f, "HID error: {e}"),
        }
    }
}

impl From<hidapi::HidError> for Error {
    fn from(e: hidapi::HidError) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
pub struct BaseInfo {
    pub profile: u8,
    pub profile_count: u8,
}

/// Something the mouse reported on its own.
#[derive(Debug, Clone, Copy)]
pub enum Notification {
    /// The active profile changed, e.g. via the button on the mouse.
    Profile(u8),
    /// The link state changed (mouse woke up, went to sleep, …).
    Status { connected: bool },
}

pub struct Mouse {
    dev: HidDevice,
    /// Packets that arrived while waiting for a specific reply.
    backlog: VecDeque<Vec<u8>>,
}

impl Mouse {
    /// Opens the first Keychron device exposing the vendor collection.
    /// Matches on VID + usage page rather than PID, since the PID can differ per receiver/cable.
    pub fn open(api: &HidApi) -> Option<Mouse> {
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == VENDOR_ID && d.usage_page() == USAGE_PAGE)?;
        let dev = info.open_device(api).ok()?;
        Some(Mouse {
            dev,
            backlog: VecDeque::new(),
        })
    }

    pub fn base_info(&mut self) -> Result<BaseInfo> {
        let d = self.request(OUT_GET, &[OP_BASE_INFO], |d| d[0] == OP_BASE_INFO)?;
        Ok(BaseInfo {
            profile: d[1],
            profile_count: d[50],
        })
    }

    /// `count` must be the current profile count: the same command also sets it.
    pub fn set_profile(&mut self, index: u8, count: u8) -> Result<()> {
        self.request(OUT_SET, &[OP_SELECT_PROFILE, index, count], |d| {
            d[0] == ACK && d[2] == OP_SELECT_PROFILE
        })?;
        Ok(())
    }

    /// Returns the next unsolicited notification, waiting at most `timeout`.
    pub fn poll(&mut self, timeout: Duration) -> Result<Option<Notification>> {
        let deadline = Instant::now() + timeout;
        loop {
            let packet = match self.backlog.pop_front() {
                Some(p) => p,
                None => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    match self.read(left)? {
                        Some(p) => p,
                        None => return Ok(None),
                    }
                }
            };
            if let Some(n) = parse_notification(&packet) {
                return Ok(Some(n));
            }
        }
    }

    fn request(
        &mut self,
        report_id: u8,
        payload: &[u8],
        matches: impl Fn(&[u8]) -> bool,
    ) -> Result<Vec<u8>> {
        for _ in 0..=RETRIES {
            self.write(report_id, payload)?;
            let deadline = Instant::now() + TIMEOUT;
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    break;
                }
                match self.read(left)? {
                    Some(p) if matches(&p) => return Ok(p),
                    Some(p) => self.backlog.push_back(p),
                    None => break,
                }
            }
        }
        Err(Error::Timeout)
    }

    fn write(&self, report_id: u8, payload: &[u8]) -> Result<()> {
        // Native backends want the report ID first and the full report length.
        let size = if report_id == OUT_SET { 20 } else { 63 };
        let mut buf = vec![0u8; size + 1];
        buf[0] = report_id;
        buf[1..=payload.len()].copy_from_slice(payload);
        self.dev.write(&buf)?;
        Ok(())
    }

    /// Reads one input report and returns its payload (report ID stripped), padded to 64 bytes
    /// so callers can index fixed offsets.
    fn read(&self, timeout: Duration) -> Result<Option<Vec<u8>>> {
        let mut buf = [0u8; 65];
        let n = self
            .dev
            .read_timeout(&mut buf, timeout.as_millis() as i32)?;
        if n == 0 {
            return Ok(None);
        }
        // Windows/Linux prefix numbered reports with their ID; strip it when present.
        let start = usize::from(matches!(buf[0], IN_SET | IN_GET));
        let mut payload = buf[start..n].to_vec();
        payload.resize(64, 0);
        Ok(Some(payload))
    }
}

fn parse_notification(d: &[u8]) -> Option<Notification> {
    match d[0] {
        NOTIFY_PROFILE => Some(Notification::Profile(d[1])),
        NOTIFY_STATUS => Some(Notification::Status {
            connected: d[2] == 1,
        }),
        _ => None,
    }
}
