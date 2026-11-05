//! Time helpers (unix seconds everywhere — no wall-clock string parsing).

use std::time::{SystemTime, UNIX_EPOCH};

/// Current unix time in whole seconds.
pub fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Current unix time in milliseconds.
pub fn unix_now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_moves_forward() {
        let a = unix_now_millis();
        std::thread::sleep(std::time::Duration::from_millis(2));
        assert!(unix_now_millis() >= a + 2);
        assert!(unix_now() > 1_700_000_000);
    }
}
