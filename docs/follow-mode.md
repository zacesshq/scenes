# Follow mode

Showing how each command typed in a shell ended: what it shows, the shell hook, how a scene finds
its shell, typos, the keys and the settings. The tones are described in [sound sets](sounds.md).
The scenes, the status bar and notifications work as in [watch mode](watch-mode.md).

`zacess-scenes follow` reacts to every command that finishes in a shell, in any language and with
any tool: `npm test`, `python3 main.py`, `make`, `git push` or `ls`. It runs nothing itself.

- **Passing:** a command that worked shows a calm scene and ALL GOOD.
- **Failing:** a command that failed shows a fail scene, FAILED, the command and its exit code.
- **Typo:** a command the shell could not find shows TYPO and `command not found` with the word it
  could not find. It plays a soft tone of its own rather than the fail phrase.
- **Recovered:** the next command that works after a failure or a typo plays the recovered tone,
  opens the calm scene out over the fail scene and reads `recovered` for a few seconds.

Every command counts, quick ones included. A command stopped with `Ctrl+C` or `Ctrl+Z` is left out,
since stopping it was a choice rather than a failure. An empty line is left out too.

## Add the hook

The shell writes one line for each command that finishes: its exit code, how long it ran and the
command. Load the hook from the shell's startup file:

| Shell | Add to | Line |
| --- | --- | --- |
| zsh | `~/.zshrc` | `eval "$(zacess-scenes follow init zsh)"` |
| bash | `~/.bashrc` | `eval "$(zacess-scenes follow init bash)"` |
| fish | `~/.config/fish/config.fish` | `zacess-scenes follow init fish \| source` |
| PowerShell | `$PROFILE` | `zacess-scenes follow init powershell \| Out-String \| Invoke-Expression` |

The hook adds almost nothing to each prompt and keeps the shell's own exit code, so the prompt
and anything else that reads it are unchanged. In zsh it runs first among the prompt hooks for that
reason. bash times commands to the whole second and needs version 4.4 or later to time them at
all. The bash that comes with macOS is 3.2, where every command is still reported but timed from
the shell's start, so `--min-seconds` needs a newer bash such as Homebrew's.

## Use follow mode

```sh
zacess-scenes follow                      # follow the shells in this tmux session
zacess-scenes follow --channel work       # follow the shells that set ZACESS_FOLLOW=work
zacess-scenes follow --min-seconds 3      # leave out commands quicker than three seconds
zacess-scenes follow --scene koi-pond,tea --sounds marimba --ambient
```

A shell and a scene meet on a channel:

1. `ZACESS_FOLLOW` when it is set in both (`--channel` names it for the scene).
2. Otherwise the tmux session's name, so a scene in one pane follows every shell in the same
   session.
3. Otherwise `default`, so outside tmux a scene follows every shell on the machine that has no
   channel of its own.

Each channel is a small log in `$XDG_STATE_HOME/zacess/follow` (`~/.local/state/zacess/follow` when
that is not set) or `%LOCALAPPDATA%\zacess\follow` on Windows. A scene reads only what is written
after it starts. It empties a log that has grown past 256 KB.

The status bar reads `following │ the dev session` and the state, with `your shell` on the default
channel. Before the first command finishes it reads `waiting`.

## Keys in follow mode

| Key | Does |
| --- | --- |
| `n` | Shows the next scene for the current state |
| `b` or `N` | Shows the scene before |
| `m` | Moves to the next sound set and plays its pass tone |
| `+` and `-` | Raise and lower the volume by 10 |
| `s` | Turns sound on or off |
| `a` | Turns the [background sound](sounds.md#background-sound) on or off |
| `[` and `]` | Lower and raise the background sound's volume by 10 |
| `p` or `Space` | Pauses and resumes the animation. The state keeps updating |
| `h` or `?` | Shows the keys again |
| `q`, `Esc` or `Ctrl+C` | Quits and gives the terminal back |

## Settings

| Setting | Flag | Config |
| --- | --- | --- |
| The channel | `--channel <name>` | `ZACESS_FOLLOW` in the environment |
| Leave out quick commands | `--min-seconds <n>` | `min_seconds` under `[follow]`, 0 by default |
| The typo tone | | `typo` under `[sound.events]`, on by default |

The scene lists, rotation, sounds, quiet hours, notifications and the background sound use the same
settings as the [watch mode](watch-mode.md) and [configuration](configuration.md).
