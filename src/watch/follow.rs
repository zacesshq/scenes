//! Following a shell: a small hook in the shell writes one line for each command that finishes,
//! with its exit code, how long it ran and the command itself. The follow mode reads those lines
//! as results. The hook and the scene find each other through a channel name, which is the
//! tmux session when there is one, so the scene beside a shell in tmux follows that shell.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::report::{Item, Kind, Report};

use super::bacon::Event;

/// The variable that names the channel, overriding the tmux session.
pub const CHANNEL_ENV: &str = "ZACESS_FOLLOW";
/// The channel when neither the variable nor tmux names one.
pub const DEFAULT_CHANNEL: &str = "default";
/// The exit code a shell gives a command it could not find.
pub const NOT_FOUND: i64 = 127;
/// A log this large is emptied when a follow starts, so it never grows without end.
const MAX_LOG: u64 = 256 * 1024;
/// The shortest time between two results. Commands pasted together can finish within
/// milliseconds of each other and each result can replace the whole picture, so only the latest
/// is shown and never more often than this, keeping to three changes a second at most.
pub const MIN_GAP: Duration = Duration::from_millis(400);

/// The shells the hook is written for.
pub const SHELLS: [&str; 4] = ["zsh", "bash", "fish", "powershell"];

/// A channel name made safe for a file name.
pub fn clean_channel(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() || cleaned.chars().all(|c| c == '.') {
        DEFAULT_CHANNEL.to_string()
    } else {
        cleaned
    }
}

/// The channel: the variable, then the tmux session, then the default.
pub fn channel(var: impl Fn(&str) -> Option<String>) -> String {
    if let Some(name) = var(CHANNEL_ENV).filter(|v| !v.trim().is_empty()) {
        return clean_channel(&name);
    }
    if var("TMUX").is_some_and(|v| !v.is_empty()) {
        let session = std::process::Command::new("tmux")
            .args(["display-message", "-p", "#S"])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|s| !s.is_empty());
        if let Some(session) = session {
            return clean_channel(&session);
        }
    }
    DEFAULT_CHANNEL.to_string()
}

/// The folder the logs live in: the state folder on macOS and Linux, local app data on Windows.
pub fn log_dir_from(var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    if cfg!(windows) {
        return var("LOCALAPPDATA").map(|d| PathBuf::from(d).join("zacess").join("follow"));
    }
    var("XDG_STATE_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| var("HOME").map(|h| PathBuf::from(h).join(".local").join("state")))
        .map(|d| d.join("zacess").join("follow"))
}

/// The log a channel's hook writes to.
pub fn log_path(dir: &Path, channel: &str) -> PathBuf {
    dir.join(format!("{}.log", clean_channel(channel)))
}

/// One finished command as the hook wrote it.
#[derive(Clone, Debug, PartialEq)]
pub struct Finished {
    pub code: i64,
    pub seconds: f64,
    pub command: String,
}

/// Reads one log line: the exit code, the seconds and the command, separated by tabs. A line
/// that is not in that shape is skipped rather than shown.
pub fn parse_line(line: &str) -> Option<Finished> {
    let mut parts = line.splitn(3, '\t');
    let code = parts.next()?.trim().parse::<i64>().ok()?;
    let seconds = parts
        .next()?
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|s| s.is_finite() && *s >= 0.0)?;
    let command = parts.next().unwrap_or("").trim().to_string();
    if command.is_empty() {
        return None;
    }
    Some(Finished {
        code,
        seconds,
        command,
    })
}

/// Whether a command was stopped by the person rather than failing: Ctrl+C or Ctrl+Z.
pub fn interrupted(code: i64) -> bool {
    matches!(code, 130 | 146 | 148 | -1073741510)
}

/// The first word of a command, which is what a shell says it could not find.
fn first_word(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or(command)
}

/// The report for one finished command.
pub fn report_for(finished: &Finished) -> Report {
    if finished.code == 0 {
        return Report::default();
    }
    let message = if finished.code == NOT_FOUND {
        format!("command not found: {}", first_word(&finished.command))
    } else {
        format!("exited with code {}", finished.code)
    };
    Report {
        errors: 1,
        warnings: 0,
        test_fails: 0,
        exit_code: Some(finished.code),
        items: vec![Item {
            kind: Kind::Error,
            message,
            location: Some(finished.command.chars().take(200).collect()),
        }],
    }
}

/// Whether a report is a command that was not found.
pub fn is_typo(report: &Report) -> bool {
    report.exit_code == Some(NOT_FOUND)
}

/// A shell being followed.
pub struct Follow {
    path: PathBuf,
    offset: u64,
    partial: String,
    min_seconds: f64,
    /// The latest result not shown yet, held while the last one is too recent.
    pending: Option<Report>,
    shown_at: Option<Instant>,
}

impl Follow {
    /// Starts reading the channel's log from its end, so commands from before are not shown.
    pub fn start(path: &Path, min_seconds: f64) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut offset = file.metadata()?.len();
        if offset > MAX_LOG {
            file.set_len(0)?;
            offset = 0;
        }
        Ok(Self {
            path: path.to_path_buf(),
            offset,
            partial: String::new(),
            min_seconds,
            pending: None,
            shown_at: None,
        })
    }

    /// The latest command that finished since the last look, as a result. Interrupted commands
    /// and those quicker than the minimum are left out. A result that comes too soon after the
    /// last is held until the gap has passed and is replaced by any newer one meanwhile.
    pub fn poll(&mut self) -> Vec<Event> {
        self.poll_at(Instant::now())
    }

    fn poll_at(&mut self, now: Instant) -> Vec<Event> {
        self.read_new();
        let due = self
            .shown_at
            .is_none_or(|at| now.duration_since(at) >= MIN_GAP);
        match self.pending.take() {
            Some(report) if due => {
                self.shown_at = Some(now);
                vec![Event::Finished(report)]
            }
            held => {
                self.pending = held;
                Vec::new()
            }
        }
    }

    fn read_new(&mut self) {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            return;
        };
        let len = file.metadata().map_or(0, |m| m.len());
        if len < self.offset {
            // the log was emptied by another follow, so reading starts again from the top
            self.offset = 0;
            self.partial.clear();
        }
        if len == self.offset || file.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut bytes = Vec::new();
        if file.read_to_end(&mut bytes).is_err() {
            return;
        }
        self.offset += bytes.len() as u64;
        self.partial.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(end) = self.partial.find('\n') {
            let line: String = self.partial.drain(..=end).collect();
            let Some(finished) = parse_line(line.trim_end_matches(['\r', '\n'])) else {
                continue;
            };
            if interrupted(finished.code) || finished.seconds < self.min_seconds {
                continue;
            }
            self.pending = Some(report_for(&finished));
        }
    }
}

/// The hook for a shell, printed by `zacess-scenes follow init <shell>` and loaded from the
/// shell's startup file. Each writes `code<TAB>seconds<TAB>command` to the channel's log after a
/// command finishes. An empty line writes nothing.
pub fn hook(shell: &str) -> Option<&'static str> {
    match shell {
        "zsh" => Some(ZSH_HOOK),
        "bash" => Some(BASH_HOOK),
        "fish" => Some(FISH_HOOK),
        "powershell" | "pwsh" => Some(POWERSHELL_HOOK),
        _ => None,
    }
}

const ZSH_HOOK: &str = r#"# zacess-scenes follow: a following scene shows how each command here ended.
# Load it with: eval "$(zacess-scenes follow init zsh)"
zmodload zsh/datetime 2>/dev/null
_zacess_follow_preexec() {
  _zacess_follow_cmd=$1
  _zacess_follow_start=$EPOCHREALTIME
}
_zacess_follow_precmd() {
  local code=$?
  [[ -n $_zacess_follow_cmd ]] || return 0
  local cmd=${_zacess_follow_cmd//$'\n'/ }
  local secs=$(( EPOCHREALTIME - _zacess_follow_start ))
  unset _zacess_follow_cmd
  if [[ -z $_zacess_follow_log ]]; then
    local channel=${ZACESS_FOLLOW:-}
    [[ -z $channel && -n $TMUX ]] && channel=$(tmux display-message -p '#S' 2>/dev/null)
    channel=${channel//[^A-Za-z0-9._-]/_}
    local dir=${XDG_STATE_HOME:-$HOME/.local/state}/zacess/follow
    [[ -d $dir ]] || mkdir -p "$dir" 2>/dev/null
    _zacess_follow_log=$dir/${channel:-default}.log
  fi
  print -r -- "$code	${secs}	${cmd//	/ }" >>| "$_zacess_follow_log" 2>/dev/null
  return 0
}
autoload -Uz add-zsh-hook
add-zsh-hook preexec _zacess_follow_preexec
precmd_functions=(_zacess_follow_precmd ${precmd_functions:#_zacess_follow_precmd})
"#;

const BASH_HOOK: &str = r#"# zacess-scenes follow: a following scene shows how each command here ended.
# Load it with: eval "$(zacess-scenes follow init bash)"
# On bash 4.4 or later PS0, drawn just before a command runs, marks that a command ran and when it
# started, so a command left out of the history (ignoredups, ignorespace, HISTIGNORE) still counts.
# The arithmetic sits on SECONDS, which is always set, since bash skips it for an unset variable.
# Older bash has no PS0, so a new history entry marks a command there, timed from the shell's start.
_zacess_follow_last=$(HISTTIMEFORMAT= builtin history 1 2>/dev/null | awk '{print $1}')
_zacess_follow_start=$SECONDS
_zacess_follow_ran=0
if (( BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4) )); then
  _zacess_follow_ps0=1
  if [[ $PS0 != *_zacess_follow_start* ]]; then
    PS0='${SECONDS:0:$((_zacess_follow_start=SECONDS,_zacess_follow_ran=1,0))}'"${PS0:-}"
  fi
fi
_zacess_follow_hook() {
  local code=$?
  local line num cmd
  line=$(HISTTIMEFORMAT= builtin history 1 2>/dev/null)
  num=${line%%[^0-9 ]*}
  num=${num// /}
  if [[ -n $_zacess_follow_ps0 ]]; then
    (( _zacess_follow_ran )) || return $code
    _zacess_follow_ran=0
  elif [[ -z $num || $num == "$_zacess_follow_last" ]]; then
    return $code
  fi
  if [[ -n $num && $num == "$_zacess_follow_last" ]]; then
    # the command ran but was kept out of the history, so its text is not known
    cmd="a command kept out of the history"
  else
    cmd=${line#*[0-9]  }
  fi
  _zacess_follow_last=$num
  cmd=${cmd//$'\n'/ }
  cmd=${cmd//$'\t'/ }
  if [[ -z $_zacess_follow_log ]]; then
    local channel=${ZACESS_FOLLOW:-}
    [[ -z $channel && -n $TMUX ]] && channel=$(tmux display-message -p '#S' 2>/dev/null)
    channel=${channel//[^A-Za-z0-9._-]/_}
    local dir=${XDG_STATE_HOME:-$HOME/.local/state}/zacess/follow
    [[ -d $dir ]] || mkdir -p "$dir" 2>/dev/null
    _zacess_follow_log=$dir/${channel:-default}.log
  fi
  printf '%s\t%s\t%s\n' "$code" "$((SECONDS - _zacess_follow_start))" "$cmd" >> "$_zacess_follow_log" 2>/dev/null
  return $code
}
if [[ $PROMPT_COMMAND != *_zacess_follow_hook* ]]; then
  PROMPT_COMMAND="_zacess_follow_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
fi
"#;

const FISH_HOOK: &str = r#"# zacess-scenes follow: a following scene shows how each command here ended.
# Load it with: zacess-scenes follow init fish | source
function _zacess_follow_postexec --on-event fish_postexec
    set -l code $status
    test -n "$argv[1]"; or return
    if not set -q _zacess_follow_log
        set -l channel $ZACESS_FOLLOW
        if test -z "$channel"; and set -q TMUX
            set channel (tmux display-message -p '#S' 2>/dev/null)
        end
        set channel (string replace -ra '[^A-Za-z0-9._-]' _ -- "$channel")
        test -n "$channel"; or set channel default
        set -l base $XDG_STATE_HOME
        test -n "$base"; or set base $HOME/.local/state
        mkdir -p $base/zacess/follow 2>/dev/null
        set -g _zacess_follow_log $base/zacess/follow/$channel.log
    end
    set -l cmd (string join ' ' -- (string split \n -- $argv[1]))
    set cmd (string replace -a \t ' ' -- $cmd)
    printf '%s\t%s\t%s\n' $code (math $CMD_DURATION / 1000) $cmd >> $_zacess_follow_log 2>/dev/null
end
"#;

const POWERSHELL_HOOK: &str = r#"# zacess-scenes follow: a following scene shows how each command here ended.
# Load it with: zacess-scenes follow init powershell | Out-String | Invoke-Expression
if (-not $global:ZacessFollowPrompt) {
    $global:ZacessFollowLastId = (Get-History -Count 1).Id
    $global:ZacessFollowLastError = if ($global:Error.Count) { $global:Error[0] } else { $null }
    $global:ZacessFollowPrompt = $function:prompt
}
function global:prompt {
    $ok = $?
    $exit = $global:LASTEXITCODE
    try {
        $last = Get-History -Count 1
        if ($last -and $last.Id -ne $global:ZacessFollowLastId) {
            $global:ZacessFollowLastId = $last.Id
            # only a new error says anything about this command, since one left from an earlier
            # command is stale. The newest error is compared by identity rather than counted,
            # since the list stops growing once it is full. A native program adds no error, so
            # its exit code is used instead
            $newError = $global:Error.Count -gt 0 -and
                -not [object]::ReferenceEquals($global:Error[0], $global:ZacessFollowLastError)
            $code = 0
            if (-not $ok) {
                if ($newError -and $global:Error[0].FullyQualifiedErrorId -eq 'CommandNotFoundException') {
                    $code = 127
                } elseif ($newError -or -not $exit) {
                    $code = 1
                } else {
                    $code = $exit
                }
            }
            if (-not $global:ZacessFollowLog) {
                $channel = $env:ZACESS_FOLLOW
                if (-not $channel -and $env:TMUX) { $channel = (tmux display-message -p '#S' 2>$null) }
                if (-not $channel) { $channel = 'default' }
                $channel = $channel -replace '[^A-Za-z0-9._-]', '_'
                if ($null -eq $IsWindows -or $IsWindows) {
                    $dir = Join-Path $env:LOCALAPPDATA 'zacess\follow'
                } else {
                    $base = if ($env:XDG_STATE_HOME) { $env:XDG_STATE_HOME } else { Join-Path $HOME '.local/state' }
                    $dir = Join-Path $base 'zacess/follow'
                }
                New-Item -ItemType Directory -Force -Path $dir | Out-Null
                $global:ZacessFollowLog = Join-Path $dir "$channel.log"
            }
            $secs = ($last.EndExecutionTime - $last.StartExecutionTime).TotalSeconds
            $cmd = ($last.CommandLine -replace "[`r`n`t]", ' ')
            $line = "{0}`t{1}`t{2}" -f $code, $secs.ToString([cultureinfo]::InvariantCulture), $cmd
            Add-Content -LiteralPath $global:ZacessFollowLog -Value $line -Encoding utf8
        }
    } catch {}
    $global:ZacessFollowLastError = if ($global:Error.Count) { $global:Error[0] } else { $null }
    # the exit code is put back, so the prompt and anything else see the command's own
    $global:LASTEXITCODE = $exit
    & $global:ZacessFollowPrompt
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_come_from_the_variable_then_the_default() {
        let none = |_: &str| None;
        assert_eq!(channel(none), DEFAULT_CHANNEL);
        let named = |k: &str| (k == CHANNEL_ENV).then(|| "my work/2".to_string());
        assert_eq!(channel(named), "my_work_2");
        assert_eq!(clean_channel(".."), DEFAULT_CHANNEL);
        assert_eq!(clean_channel(""), DEFAULT_CHANNEL);
    }

    #[test]
    fn the_log_lives_in_the_state_folder() {
        if cfg!(windows) {
            return;
        }
        let var = |k: &str| (k == "HOME").then(|| "/home/x".to_string());
        assert_eq!(
            log_dir_from(var).unwrap(),
            PathBuf::from("/home/x/.local/state/zacess/follow")
        );
        let xdg = |k: &str| (k == "XDG_STATE_HOME").then(|| "/s".to_string());
        assert_eq!(
            log_dir_from(xdg).unwrap(),
            PathBuf::from("/s/zacess/follow")
        );
    }

    #[test]
    fn lines_parse_and_bad_ones_are_skipped() {
        assert_eq!(
            parse_line("1\t2.5\tnpm test -- --watch=false"),
            Some(Finished {
                code: 1,
                seconds: 2.5,
                command: "npm test -- --watch=false".to_string(),
            })
        );
        assert_eq!(parse_line("0\t0,25\tls").unwrap().seconds, 0.25);
        assert_eq!(parse_line("x\t1\tls"), None);
        assert_eq!(parse_line("0\t1\t"), None);
        assert_eq!(parse_line("0\t-1\tls"), None);
    }

    #[test]
    fn a_typo_names_the_word_and_a_failure_its_code() {
        let typo = report_for(&parse_line("127\t0.01\tgti status").unwrap());
        assert!(is_typo(&typo));
        assert_eq!(typo.items[0].message, "command not found: gti");
        assert_eq!(typo.items[0].location.as_deref(), Some("gti status"));
        let fail = report_for(&parse_line("2\t4\tmake").unwrap());
        assert!(fail.failed() && !is_typo(&fail));
        assert_eq!(fail.items[0].message, "exited with code 2");
        assert!(!report_for(&parse_line("0\t1\tls").unwrap()).failed());
    }

    #[test]
    fn following_reads_only_new_commands_and_skips_interruptions() {
        let dir = std::env::temp_dir().join(format!("zacess-follow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = log_path(&dir, "test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "1\t1\told failure\n").unwrap();
        let mut follow = Follow::start(&path, 0.0).unwrap();
        assert!(follow.poll().is_empty(), "history is not shown");
        let append = |text: &str| {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            f.write_all(text.as_bytes()).unwrap();
        };
        let start = Instant::now();
        append("0\t0.1\tls\n130\t3\tsleep 9\n127\t0.0\tgti\n0\t2");
        let events = follow.poll_at(start);
        assert_eq!(
            events.len(),
            1,
            "only the latest of a burst shows: {events:?}"
        );
        assert!(matches!(&events[0], Event::Finished(r) if is_typo(r)));
        append("\tcargo build\n");
        assert!(
            follow
                .poll_at(start + Duration::from_millis(100))
                .is_empty(),
            "a result too soon after the last is held"
        );
        let later = follow.poll_at(start + MIN_GAP);
        assert!(
            matches!(&later[..], [Event::Finished(r)] if !r.failed()),
            "a line written in two parts is read once whole and shown after the gap"
        );

        let mut slow_only = Follow::start(&path, 3.0).unwrap();
        append("1\t4\tslow\n1\t0.5\tquick\n");
        assert!(matches!(&slow_only.poll()[..], [Event::Finished(r)] if r.failed()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_shell_has_a_hook_that_writes_the_three_fields() {
        for shell in SHELLS {
            let hook = hook(shell).unwrap();
            assert!(hook.contains("zacess/follow") || hook.contains("zacess\\follow"));
            assert!(hook.contains("ZACESS_FOLLOW"), "{shell}");
        }
        assert!(hook("tcsh").is_none());
    }

    #[test]
    fn interruptions_are_recognised() {
        assert!(interrupted(130));
        assert!(!interrupted(1));
        assert!(!interrupted(127));
    }
}
