# Watch mode

The build monitor for projects in any language: what it shows, the command each kind of project
runs, bacon for Rust, the status bar, choosing the scenes, the keys and desktop notifications. The
tones are described in [sound sets](sounds.md) and every setting in
[configuration](configuration.md). To show how each command typed in a shell ended instead, see
[follow mode](follow-mode.md).

`zacess-scenes watch` shows how the project in the current folder stands. It runs the project's
check once at the start and again whenever a file changes. A Rust project uses
[bacon](https://dystroy.org/bacon/) when it is installed; every other project runs its own command
and its exit code decides pass or fail:

- **Passing:** a calm scene, `waves` unless you choose others, with a large ALL GOOD banner, a check
  mark and a short caption that belongs to the scene. Any warnings are listed below it.
- **Failing:** a fail scene, `storm` unless you choose others, with a large banner, a cross, a
  caption and the first errors or failing tests, each with its location where one is given. The
  banner reads BUILD FAILED with bacon and FAILED for any other command. Warnings follow the
  errors.
- **Recovered:** when a failing build is fixed, the pass scene opens out from the middle of the
  screen over the fail scene in two seconds, its edge traced by a ring of small sparkles. The status
  bar reads `recovered` and a recovered caption shows for a few seconds. Under reduced motion the
  opening takes three and a half seconds and has no sparkles. Nothing flashes.
- **Warnings** are marked `!` and drawn in the theme's warning colour. Errors and failing tests are
  marked with a cross and drawn in the theme's error colour, so the two differ in shape as well as
  colour. Only bacon reports warnings, so the count shows only for Rust projects watched with it.

## Use watch mode

```sh
cd my-project
zacess-scenes watch                          # the project's own check, in any language
zacess-scenes watch --cmd "javac Main.java"  # any command; its exit code decides
zacess-scenes watch test --volume 30         # bacon's test job, with quieter tones
zacess-scenes watch --no-sound               # no tones at all
zacess-scenes watch --scene koi-pond,tea --fail-scene flood --sounds marimba --notify
```

## The command for each project

The nearest folder at or above the current one with one of these files decides the command. It runs
through the system shell (`sh` on macOS and Linux, `cmd` on Windows) in that folder.

| The folder has | Runs |
| --- | --- |
| `Cargo.toml` | bacon when it is installed (see below), otherwise `cargo check` |
| `package.json` | The `check` script, then `test` (unless it is the placeholder `npm init` writes), then `build`, then `lint`, with `pnpm`, `yarn` or `bun` when their lockfile is there and `npm` otherwise |
| `go.mod` | `go build ./... && go vet ./...` |
| `pyproject.toml`, `setup.py`, `setup.cfg` or `requirements.txt` | `python3 -m pytest -q` when there are tests (`tests/`, `test/` or `test_*.py`), otherwise `python3 -m compileall -q .`, which still catches a syntax error. `python` on Windows |
| `pom.xml` | `mvn -q compile` |
| `build.gradle` or `build.gradle.kts` | The Gradle wrapper's `build` when the project has one, otherwise `gradle -q build` |
| A `.sln`, `.csproj` or `.fsproj` file | `dotnet build` |
| `Package.swift` | `swift build` |
| `mix.exs` | `mix compile` |
| `composer.json` with a `test` script | `composer test` |
| `Gemfile` and `Rakefile` | `bundle exec rake` |
| `Makefile` | `make` |
| `CMakeLists.txt` and a `build` folder | `cmake --build build` |

`--cmd` runs any other command instead, from the nearest project folder or the current one. A
command's tools have to be installed: pytest for Python tests, for example. When they are not, the
failure screen shows the tool's own message, such as `No module named pytest`.

Files are looked at for a change every 0.7 seconds and a run starts once they have been still for
0.3 seconds, so a save that writes several files starts one run. Folders that hold build output,
dependencies or tool state, such as `.git`, `target`, `node_modules`, `dist`, `build`,
`__pycache__` and `.venv`, are not watched, since a change there is usually the command's own
doing. A run under way is stopped when another starts and when the watch mode quits, along with
anything it started.

The lines shown for a failure are the ones that read as errors, such as `error:`, `FAILED`,
`AssertionError` or `panicked at`, with a place such as `src/app.py:12` split out where a line
starts with one. When no line looks like an error, the last line of output is shown, so a failure
always says something.

## Rust and bacon

bacon is a separate program, not part of Zacess Scenes (see [NOTICE.md](../NOTICE.md)). Install it
once:

```sh
cargo install --locked bacon
```

Without bacon a Rust project runs `cargo check` like any other command. Naming one of bacon's jobs
asks for bacon, so without it `zacess-scenes watch clippy` prints the install command and stops.

| Job | Runs |
| --- | --- |
| `check` (the default) | `cargo check` |
| `check-all` | `cargo check --all-targets` |
| `clippy` | `cargo clippy` |
| `test` | `cargo test` |

With bacon the number keys switch the job. bacon reruns the job whenever a file in the project
changes. Its JSON report export is turned on
through bacon's `--config-toml` option and written to a temporary folder that is removed on quit.
The project's own `bacon.toml` is read as usual and never written to.

The bottom row is a status bar in the theme's colours:

```text
my-project │ check │ ✓ passing · 2 warnings │ 12s ago
```

While the build fails it reads `✗ failing` in the theme's error colour with the count of errors or
failed tests. Outside bacon the job is the command itself. While a run is under way it adds
`· running`. Notes such as `paused`,
`sound off` and `no sound player` follow at the end. On a narrow terminal the notes go first, then
the age and then the job.

`--theme`, `--fps`, `--motion`, `--reduced-motion` and `--colour` work as they do for the scenes.
Under reduced motion the storm has no lightning. A change of scene, sound set or volume is written
in the status bar for three seconds, such as `scene koi-pond · sounds marimba`. The status bar
shows `quiet hours` while [quiet hours](configuration.md) keep the sound off.

## Choosing the scenes

| Flag | Variable | Config | Does |
| --- | --- | --- | --- |
| `--scene <names>` | `ZACESS_SCENE` | `pass` | The scenes to show while the build passes, separated by commas. Pass and general scenes and your own Lua scenes |
| `--fail-scene <names>` | `ZACESS_FAIL_SCENE` | `fail` | The scenes to show while it fails. Fail scenes and your own Lua scenes |
| | | `rotation` | `fixed` keeps the first scene, `cycle` moves to the next scene in the list each time the state comes round again and `shuffle` picks another at random, never the same twice running |
| | | `interval` | With `cycle` or `shuffle`, also change the scene every this many minutes while the state stays the same. 0, the default, changes only on a new result |

Without a list, the pass list is `waves` followed by every other pass and general scene and the
fail list is `storm` followed by every other fail scene. `n` and `b` always move through the list
showing; a list of one scene widens to every scene of its kind, so the keys always have somewhere
to go.

## Keys in watch mode

| Key | Does |
| --- | --- |
| `1` to `4` | With bacon, switches to the `check`, `check-all`, `clippy` or `test` job |
| `r` | Runs the job or the command again |
| `n` | Shows the next scene for the current state |
| `b` or `N` | Shows the scene before |
| `m` | Moves to the next sound set and plays its pass tone |
| `+` and `-` | Raise and lower the volume by 10. `=` and `_` work too, so Shift is not needed |
| `s` | Turns sound on or off |
| `a` | Turns the [background sound](sounds.md#background-sound) on or off |
| `[` and `]` | Lower and raise the background sound's volume by 10 |
| `p` or `Space` | Pauses and resumes the animation. The status keeps updating |
| `h` or `?` | Shows the keys again |
| `q`, `Esc` or `Ctrl+C` | Quits, stops bacon or the command and gives the terminal back |

A scene changes at most once every 0.4 seconds however fast `n` or `b` is pressed, so the picture
is never replaced faster than three times a second.

## Notifications

`--notify` sends a desktop notification when a build starts failing and when it is fixed, such as
`my-project: check failed with 2 problems.` Screen readers announce notifications, so the state can
be heard without looking at the terminal. They use the system's own tools:

| System | Sent with |
| --- | --- |
| macOS | `osascript` and `display notification` |
| Linux | `notify-send` |
| Windows | A PowerShell toast or a tray balloon where the toast is not available. No module is needed |

Notifications are off by default. Turn them on with `--notify`, `ZACESS_NOTIFY=1` or `enabled =
true` under `[notifications]` in the config. `--no-notify` turns them off again for one run.
