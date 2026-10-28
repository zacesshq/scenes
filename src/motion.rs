//! Reduced motion: the flag, the `ZACESS_REDUCED_MOTION` variable and the operating system's own
//! setting, read without any extra dependency.

use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long a settings command may take. A command can hang, as `gsettings` does without a
/// session bus under WSL. The scene should start regardless.
const COMMAND_TIMEOUT: Duration = Duration::from_millis(500);

/// The environment variable that turns reduced motion on.
pub const ENV_VAR: &str = "ZACESS_REDUCED_MOTION";

/// Whether the variable's value asks for reduced motion: `1`, `true`, `yes` or `on`.
pub fn env_wants_reduced(value: Option<&str>) -> bool {
    matches!(
        value.map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

/// Combines the three sources. The system is only asked when neither the flag nor the variable
/// has already decided.
pub fn reduced(flag: bool, env: Option<&str>, system: impl FnOnce() -> bool) -> bool {
    flag || env_wants_reduced(env) || system()
}

/// Runs a command and returns its standard output (`None` when it is missing, fails or takes
/// longer than the timeout, which counts as the setting being unknown).
#[cfg_attr(
    not(any(target_os = "macos", target_os = "linux", target_os = "windows")),
    allow(dead_code)
)]
fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let read = std::io::Read::read_to_string(&mut { stdout }, &mut text);
        let _ = send.send(read.map(|_| text));
    });
    match receive.recv_timeout(COMMAND_TIMEOUT) {
        Ok(Ok(text)) => child
            .wait()
            .ok()
            .filter(|status| status.success())
            .map(|_| text),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    }
}

/// Whether a Windows `UserPreferencesMask` value has client area animation turned off, the
/// Settings switch Accessibility, Visual effects, Animation effects. The flag is bit 1 of the
/// fifth byte.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn animation_effects_off(mask: &str) -> bool {
    let hex = mask.trim().trim_start_matches("0x");
    hex.get(8..10)
        .and_then(|byte| u8::from_str_radix(byte, 16).ok())
        .is_some_and(|byte| byte & 0x02 == 0)
}

/// Reads one value from the Windows registry through `reg query` and returns its data column.
#[cfg(target_os = "windows")]
fn registry_value(key: &str, name: &str) -> Option<String> {
    let output = command_output("reg", &["query", key, "/v", name])?;
    output
        .lines()
        .find(|line| line.trim_start().starts_with(name))
        .and_then(|line| line.split_whitespace().last())
        .map(str::to_string)
}

/// Whether the operating system asks for reduced motion: Reduce Motion on macOS, animations
/// turned off in GNOME or in the Windows visual effects.
pub fn system_reduced_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        command_output(
            "defaults",
            &["read", "com.apple.universalaccess", "reduceMotion"],
        )
        .is_some_and(|out| out.trim() == "1")
    }
    #[cfg(target_os = "linux")]
    {
        command_output(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "enable-animations"],
        )
        .is_some_and(|out| out.trim() == "false")
    }
    #[cfg(target_os = "windows")]
    {
        let minimise_off =
            registry_value(r"HKCU\Control Panel\Desktop\WindowMetrics", "MinAnimate")
                .is_some_and(|value| value == "0");
        minimise_off
            || registry_value(r"HKCU\Control Panel\Desktop", "UserPreferencesMask")
                .is_some_and(|mask| animation_effects_off(&mask))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

/// Whether the operating system is in dark mode, used when the theme state file follows the
/// system. Unknown counts as dark, the default theme's appearance.
pub fn system_prefers_dark() -> bool {
    #[cfg(target_os = "macos")]
    {
        command_output("defaults", &["read", "-g", "AppleInterfaceStyle"])
            .is_some_and(|out| out.trim().eq_ignore_ascii_case("dark"))
    }
    #[cfg(target_os = "linux")]
    {
        command_output(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "color-scheme"],
        )
        .is_none_or(|out| !out.contains("prefer-light") && !out.contains("'default'"))
    }
    #[cfg(target_os = "windows")]
    {
        registry_value(
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "AppsUseLightTheme",
        )
        .is_none_or(|value| value == "0x0")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variable_accepts_the_usual_spellings() {
        for value in ["1", "true", "TRUE", "yes", "on", " 1 "] {
            assert!(env_wants_reduced(Some(value)), "{value}");
        }
        for value in ["0", "false", "no", "off", ""] {
            assert!(!env_wants_reduced(Some(value)), "{value}");
        }
        assert!(!env_wants_reduced(None));
    }

    #[test]
    fn any_source_turns_reduced_motion_on() {
        assert!(reduced(true, None, || false));
        assert!(reduced(false, Some("1"), || false));
        assert!(reduced(false, None, || true));
        assert!(!reduced(false, Some("0"), || false));
    }

    #[test]
    fn the_system_is_not_asked_when_the_flag_decides() {
        let mut asked = false;
        assert!(reduced(true, None, || {
            asked = true;
            true
        }));
        assert!(!asked);
    }

    #[test]
    fn the_windows_animation_switch_is_read_from_the_mask() {
        // the defaults with animations on, then the same with bit 1 of byte 4 cleared
        assert!(!animation_effects_off("9E1E078012000000"));
        assert!(animation_effects_off("9E1E078010000000"));
        assert!(!animation_effects_off("9E"));
    }

    #[cfg(unix)]
    #[test]
    fn a_hanging_command_times_out() {
        let start = std::time::Instant::now();
        assert_eq!(command_output("sleep", &["5"]), None);
        assert!(start.elapsed() < Duration::from_secs(3));
        assert_eq!(command_output("echo", &["hi"]).as_deref(), Some("hi\n"));
    }

    #[test]
    fn a_missing_command_is_not_an_error() {
        assert_eq!(command_output("zacess-no-such-command", &[]), None);
    }
}
