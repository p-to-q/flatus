//! User-idle detection.
//!
//! "Is the user actively at the machine right now?" The pressure state machine
//! uses this as the `ActivitySignal` so personality `activity_bonus` only kicks
//! in when someone's actually typing or moving the mouse — otherwise idle
//! accumulation is at the baseline rate.
//!
//! macOS: we ask the `IOHIDSystem` registry entry for `HIDIdleTime` via the
//! `ioreg` CLI. This is fewer LOC and zero new deps versus a direct `IOKit` FFI;
//! once per pressure tick (~1 Hz) the subprocess cost is negligible.
//! TODO(v0.3): swap to direct `IOKit` + `core-foundation` FFI for the perf and
//! to drop the `/usr/sbin/ioreg` runtime dependency.
//!
//! Other platforms return `None` — the caller treats that as "no signal, fall
//! back to the personality's baseline rate".

#[cfg(target_os = "macos")]
use std::process::Command;

/// Seconds since the user last touched the keyboard or mouse, or `None` if
/// the platform has no implementation or the query failed for any reason.
#[must_use]
pub fn user_idle_secs() -> Option<u64> {
    platform_user_idle_secs()
}

#[cfg(target_os = "macos")]
fn platform_user_idle_secs() -> Option<u64> {
    // `ioreg -c IOHIDSystem -r -d 4` dumps the IOHIDSystem registry entry and
    // its first four levels of children as plain text. `HIDIdleTime` is a key
    // whose value is nanoseconds since last HID input.
    let output = Command::new("ioreg")
        .args(["-c", "IOHIDSystem", "-r", "-d", "4"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = std::str::from_utf8(&output.stdout).ok()?;
    for line in stdout.lines() {
        // Lines look like:  `    "HIDIdleTime" = 1234567890`
        if let Some(rest) = line.split_once("\"HIDIdleTime\"") {
            let value = rest.1.trim_start_matches([' ', '=']).trim();
            return value.parse::<u64>().ok().map(|nanos| nanos / 1_000_000_000);
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn platform_user_idle_secs() -> Option<u64> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn macos_idle_returns_a_value_under_a_day() {
        // We can't deterministically test the exact value (it depends on what
        // the test runner is doing) but we can sanity-check that the query
        // works and returns something within a plausible range.
        let idle = user_idle_secs();
        assert!(idle.is_some(), "expected ioreg to return an HIDIdleTime");
        assert!(idle.unwrap() < 86_400, "idle time should not exceed a day");
    }
}
