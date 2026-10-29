//! Desktop notifications for the watch mode, sent through the system's own tools: `osascript`
//! on macOS, `notify-send` on Linux and a PowerShell toast on Windows, with a tray balloon as the
//! fallback where the toast is not available. No extra module or library is needed. A screen
//! reader announces a notification, so the state is heard without looking at the terminal.

use std::process::{Command, Stdio};

/// The title every notification carries.
pub const TITLE: &str = "Zacess Scenes";

/// A state change worth a notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Passing,
    Failing,
    Recovered,
}

/// The text of a notification: what happened and to which project and job.
pub fn message(change: Change, project: &str, job: &str, problems: u64) -> String {
    match change {
        Change::Passing => format!("{project}: {job} passes. All good."),
        Change::Recovered => format!("{project}: {job} passes again. Fixed."),
        Change::Failing if problems == 1 => format!("{project}: {job} failed with 1 problem."),
        Change::Failing if problems > 1 => {
            format!("{project}: {job} failed with {problems} problems.")
        }
        Change::Failing => format!("{project}: {job} failed."),
    }
}

/// An AppleScript string literal.
fn applescript_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A PowerShell single quoted string.
fn powershell_string(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The PowerShell script for Windows: a toast through the Windows Runtime under PowerShell's own
/// app id, otherwise a balloon from a tray icon.
fn powershell_script(title: &str, body: &str) -> String {
    let title = powershell_string(title);
    let body = powershell_string(body);
    format!(
        "$t = {title}; $b = {body}; try {{ \
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
$x = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
$n = $x.GetElementsByTagName('text'); $n.Item(0).AppendChild($x.CreateTextNode($t)) > $null; \
$n.Item(1).AppendChild($x.CreateTextNode($b)) > $null; \
$id = '{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\\WindowsPowerShell\\v1.0\\powershell.exe'; \
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($id).Show([Windows.UI.Notifications.ToastNotification]::new($x)) \
}} catch {{ \
Add-Type -AssemblyName System.Windows.Forms; $i = New-Object System.Windows.Forms.NotifyIcon; \
$i.Icon = [System.Drawing.SystemIcons]::Information; $i.Visible = $true; \
$i.ShowBalloonTip(5000, $t, $b, 'Info'); Start-Sleep -Seconds 6; $i.Dispose() \
}}"
    )
}

/// The command that shows a notification on this system, if it has a way to.
pub fn command(title: &str, body: &str) -> Option<Command> {
    if cfg!(target_os = "macos") {
        let mut command = Command::new("osascript");
        command.args([
            "-e",
            &format!(
                "display notification {} with title {}",
                applescript_string(body),
                applescript_string(title)
            ),
        ]);
        Some(command)
    } else if cfg!(windows) {
        let program = crate::sound::find_program("powershell")
            .or_else(|| crate::sound::find_program("pwsh"))?;
        let mut command = Command::new(program);
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &powershell_script(title, body),
        ]);
        Some(command)
    } else {
        let program = crate::sound::find_program("notify-send")?;
        let mut command = Command::new(program);
        command.args(["--app-name", TITLE, title, body]);
        Some(command)
    }
}

/// Sends a notification in the background. A system without a way to show one stays silent:
/// the terminal always shows the same state.
pub fn send(title: &str, body: &str) -> bool {
    let Some(mut command) = command(title, body) else {
        return false;
    };
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(mut child) => {
            // waited for on its own thread so the finished command never lingers as a zombie
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_say_what_changed() {
        assert_eq!(
            message(Change::Failing, "sample", "check", 2),
            "sample: check failed with 2 problems."
        );
        assert_eq!(
            message(Change::Failing, "sample", "test", 1),
            "sample: test failed with 1 problem."
        );
        assert!(message(Change::Recovered, "a", "clippy", 0).contains("passes again"));
        assert!(message(Change::Passing, "a", "check", 0).contains("All good"));
    }

    #[test]
    fn quotes_are_escaped_for_each_system() {
        assert_eq!(
            applescript_string(r#"say "hi" \ bye"#),
            r#""say \"hi\" \\ bye""#
        );
        assert_eq!(powershell_string("o'neil"), "'o''neil'");
        let script = powershell_script("Zacess Scenes", "it's done");
        assert!(script.contains("$b = 'it''s done'"));
        assert!(script.contains("ToastNotificationManager"));
        assert!(script.contains("ShowBalloonTip"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_mac_command_uses_osascript() {
        let command = command("T", "B \"x\"").unwrap();
        assert_eq!(command.get_program(), "osascript");
        let args: Vec<String> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args[1],
            "display notification \"B \\\"x\\\"\" with title \"T\""
        );
    }
}
