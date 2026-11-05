//! Fixed-window rate limiting, per key.
//!
//! Deliberately simple: an in-memory `HashMap<key, (window_id, count)>` under
//! a std `Mutex` (critical sections are nanoseconds; no await inside).
//! Restarting the server resets the windows — acceptable for a self-hosted
//! single-node chat server and worth not shipping Redis.

use std::collections::HashMap;
use std::time::Duration;

use koko_core::time::unix_now_millis;

/// Fixed-window limiter.
#[derive(Debug, Default)]
pub struct RateLimiter {
    inner: std::sync::Mutex<HashMap<String, (u64, u32)>>,
}

/// Policy knobs (per second is not the point here; windows are minutes/hours).
pub mod policy {
    use std::time::Duration;

    /// Registration: 5 per hour per IP.
    pub const REGISTER_MAX: u32 = 5;
    /// Registration window.
    pub const REGISTER_WINDOW: Duration = Duration::from_secs(3600);

    /// Login: 10 per minute per IP.
    pub const LOGIN_IP_MAX: u32 = 10;
    /// Login: 5 per minute per handle (blocks one-account brute force).
    pub const LOGIN_HANDLE_MAX: u32 = 5;
    /// Login window.
    pub const LOGIN_WINDOW: Duration = Duration::from_secs(60);

    /// Media upload: 60 per minute per user.
    pub const MEDIA_MAX: u32 = 60;
    /// Media window.
    pub const MEDIA_WINDOW: Duration = Duration::from_secs(60);
}

impl RateLimiter {
    /// New, empty limiter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a hit for `key` and say whether it is allowed.
    pub fn allow(&self, key: &str, max: u32, window: Duration) -> bool {
        let now = unix_now_millis().max(0) as u64;
        let window_ms = window.as_millis() as u64;
        let wid = now / window_ms.max(1);
        let mut map = self.inner.lock().expect("rate limiter mutex poisoned");
        match map.get_mut(key) {
            Some(entry) if entry.0 == wid => {
                entry.1 += 1;
                entry.1 <= max
            }
            Some(entry) => {
                *entry = (wid, 1);
                true
            }
            None => {
                map.insert(key.to_owned(), (wid, 1));
                true
            }
        }
    }

    /// Drop idle windows (call occasionally; bounded map otherwise).
    pub fn sweep(&self) {
        let now = unix_now_millis().max(0) as u64;
        let mut map = self.inner.lock().expect("rate limiter mutex poisoned");
        map.retain(|_, (wid, _)| now.saturating_sub(*wid * 3_600_000) < 7_200_000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_window_behavior() {
        let rl = RateLimiter::new();
        for _ in 0..3 {
            assert!(rl.allow("k", 3, Duration::from_secs(60)));
        }
        assert!(
            !rl.allow("k", 3, Duration::from_secs(60)),
            "4th hit in window is denied"
        );
        // Different key is independent.
        assert!(rl.allow("other", 3, Duration::from_secs(60)));
    }
}
