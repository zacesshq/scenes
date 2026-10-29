# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- 16 new pass scenes: `koi-pond`, `harbour`, `lanterns`, `balloons`, `countryside`, `winter-cabin`, `autumn`, `blossom`, `city-night`, `orbit`, `constellations`, `tea`, `cat-nap`, `garden`, `tiles` and `ribbons`. With the original ten, 26 scenes can show while a build passes. Each has captions of its own.
- 12 new fail scenes that read as something gone wrong without ever flashing, shaking or spinning, namely `flood`, `listing-ship`, `corrosion`, `tangle`, `wilt`, `fog`, `broken-bridge`, `leak`, `blackout`, `traffic-jam`, `erosion` and `drought`. The storm is now a scene of its own, `storm`, so there are 13. Each progresses slowly and then holds, so it still looks right after an hour. Each has captions.
- A recovered moment: when a failing build is fixed, the pass scene opens out from the middle over the fail scene in two seconds with a ring of small sparkles, the status bar reads `recovered` and a recovered caption shows. Under reduced motion it takes three and a half seconds with no sparkles.
- Six sound sets, all synthesised at run time, namely `bells` (the original tones), `marimba`, `kalimba`, `pad`, `chip` and `glass`. Each has tones for a pass, a failure, a rebuild tick, a fix, a build that is still failing (very quiet and off by default) and new warnings. Every set plays the same phrase for each event, so the meaning carries over. Every tone is levelled to the same loudness with a peak ceiling.
- A config file at `$XDG_CONFIG_HOME/zacess/scenes.toml` or `~/.config/zacess/scenes.toml` on macOS and Linux and `%APPDATA%\zacess\scenes.toml` on Windows, read by a small TOML parser of our own. It holds the theme, the pass and fail scene lists, the rotation (fixed, cycle or shuffle) and its interval, the motion level, reduced motion, captions, the banner, sound, the sound set, the volume, quiet hours, per-event sound switches and notifications. `ZACESS_CONFIG` points at another file.
- Commands: `zacess-scenes list`, `config init` (a fully commented starter that changes nothing until a line is uncommented, refusing to replace a file without `--force`), `config path`, `sounds play <set> [event]`, `scene new <name>` and `snapshot <scene>`.
- Flags: `--scene`, `--fail-scene`, `--sounds`, `--motion` (calm, standard or lively) and `--notify` with `--no-notify`, plus the `ZACESS_SCENE`, `ZACESS_FAIL_SCENE`, `ZACESS_SOUNDS`, `ZACESS_MOTION` and `ZACESS_NOTIFY` variables. A flag wins over a variable, which wins over the config file, which wins over the default.
- Watch mode keys: `n` for the next scene, `N` or `b` for the one before, `m` for the next sound set and `+` and `-` for the volume. The status bar names the new scene, sound set or volume for three seconds. The standalone scenes take `n` and `b` too and name the new scene on the bottom row.
- Desktop notifications with `--notify`: when a build starts failing and when it is fixed, through `osascript` on macOS, `notify-send` on Linux and a PowerShell toast on Windows with a tray balloon as the fallback. A screen reader announces them, so the state can now be heard without looking at the terminal.
- Lua scenes of your own: `.lua` files in the `scenes` folder beside the config file are loaded as extra scenes with a name, a kind and captions, listed by `list` and usable everywhere a built-in scene is. Lua 5.4 is compiled into the binary through mlua, so nothing needs installing. Scripts draw with `put(x, y, text, role)`, where the role is a palette role name and never a raw colour, so every colour keeps the contrast floor. They run in a sandbox without io, os, require, load or debug, with an instruction budget per frame and a memory limit. A frame that would change a large area more than three times a second is held back, a scene that keeps flashing is stopped and any error shows in the panel and the status bar while a built-in scene takes over. Optional `on_pass`, `on_fail` and `on_recovered` hooks can return a caption. `examples/scenes/lantern.lua` is a worked example and `docs/lua.md` is the reference.
- `zacess-scenes snapshot <scene>` draws a scene with the real renderer into a self-contained SVG still in a terminal window frame with `role="img"` and an `aria-label` or, with `--frames`, into compact JSON frames for a web page to play. It works for Lua scenes and, with `--state pass` or `--state fail`, for the watch mode with sample data. Frames go through the flash meter.
- A gallery: an SVG still of every scene in `docs/images/scenes`, two watch mode shots and a Light theme example, plus showcase frames in `docs/frames`, all made by `scripts/snapshots.sh`. CI fails when they are out of date.
- A flash meter (`src/flash.rs`) that measures the brightness of each region of a frame. Every scene is now checked in all eight themes at four sizes and six motion settings for contrast, for characters every supported font has, for odd sizes and for no more than three large changes of brightness in any second.
- CI builds the release targets (Intel macOS and both musl Linux targets) on every pull request, renders the example Lua scene on Linux, macOS and Windows and checks the gallery.

### Fixed

- Closing the terminal under a running scene, such as a closed tmux pane or a dropped SSH session, no longer leaves the program spinning a processor core at 100 percent. crossterm's read loop never returns once its terminal has gone, so after a termination signal the program now gives the run loop one second to end and then ends on its own. This affected 0.1.0 and 0.2.0 too.

### Changed

- The watch mode's scene lists default to `waves` and `storm` first, then every other pass or fail scene, so `n` has somewhere to go. Captions now come from the scene showing and a short caption sits above the errors while failing.
- The lively motion level moves things faster and draws more of them but never changes anything in place faster than the standard level. The wing beats in `waves` follow that rule.
- The help, the README, `docs/scenes.md`, the accessibility statement, the contributing guide, the issue forms and the licensing notice cover the new scenes, sound sets, config, keys, notifications, Lua scenes and snapshots.

## [0.2.0] - 2026-10-29

### Added

- `zacess-scenes watch [job]`, a build monitor for Rust projects. It runs bacon headless with its JSON report export turned on through `--config-toml`, so the project's own `bacon.toml` is never written to. Each report is read with a small JSON reader of our own. The jobs are `check` (the default), `check-all`, `clippy` and `test`.
- While the build passes, the `waves` scene with a large ALL GOOD banner, a check mark, a short caption and any warnings. While it fails, a storm with a large BUILD FAILED banner, a cross and the first errors or failing tests with their locations, followed by the warnings. Warnings are marked `!` in the theme's warning colour and errors a cross in its error colour.
- A status bar in the active theme's colours, such as `project │ job │ ✓ passing · 2 warnings │ 12s ago`, with `✗ failing` in the theme's error colour, `running` while bacon runs the job and notes for a paused animation, sound turned off or no sound player. It fits every width from one column up.
- Watch mode keys: `1` to `4` switch the job, `r` reruns it, `p` or `Space` pauses the animation, `s` turns sound on or off, `h` shows the keys again and `q` quits and stops bacon.
- A clear message outside a Cargo project and another when bacon is missing, with the command `cargo install --locked bacon`.
- Tones of our own, synthesised at run time with soft bell envelopes and short fades into WAV files in the user's cache folder: a gentle rising chime when a run passes, a lower two-note tone when it fails and a soft tick when a rebuild starts. They play in the background through `afplay`, `pw-play`, `paplay`, `aplay` or PowerShell, whichever the system has. A missing player only means no sound and a note in the status bar.
- `--no-sound`, `--volume <0-100>` and the `ZACESS_SOUND` and `ZACESS_VOLUME` environment variables.
- A storm variant of the rain scene with twice the drops, a wind and a second row of cloud. It keeps the five second gap between bolts, waits five seconds before its first and has no lightning under reduced motion.
- Tests for the report reader against real bacon reports of a clean check, a warning, two errors, Clippy warnings and passing and failing tests, for the WAV header, length, level and fades and for the status bar at every width. Every colour the watch mode draws is checked against each theme's floor.

### Changed

- The README, `docs/scenes.md`, the accessibility statement, the contributing guide and the licensing notice describe the watch mode, its sounds and bacon.

## [0.1.0] - 2026-10-28

### Added

- Ten scenes: `sea`, `rain`, `snow`, `starfield`, `code-rain`, `fireflies`, `aurora`, `campfire`, `waves` and `zacess`. Each lays itself out for any terminal from 20 columns by 8 rows upwards and again on every resize.
- The eight Zacess themes from Zacess Themes 0.5.0, embedded from `palette/zacess.json` by `scripts/build-palette.py`. The theme comes from `--theme`, then `ZACESS_THEME`, then a theme state file at `$XDG_STATE_HOME/zacess/theme` (a fixed theme or `auto` with a dark and a light theme to follow the system), otherwise Zacess Deep Contrast Dark. `--list-themes` prints them and marks the active one. `NO_COLOR` draws in the terminal's own text colour.
- A contrast check in the code and in the tests: every colour a scene draws keeps at least 4.5 to 1 on the theme's background (7 to 1 on the High Contrast and Deep Contrast themes, where a weaker terminal colour is swapped for the nearest palette colour that keeps it). The Zacess mark's band is kept apart from its page and tile by a row of background, so it stays visible in every theme and under `NO_COLOR`.
- 24 bit colour where the terminal supports it and the nearest 256 colour entry that keeps the contrast floor elsewhere, with `--colour truecolor` or `--colour 256` to override.
- Keys: Space pauses and resumes, `h` shows the help line again and `q`, `Esc` or `Ctrl+C` quits. The help line shows for a few seconds on start, dims and then goes; a paused scene says so and how to resume. Key repeats are ignored and pause toggles at most every 0.3 seconds, so a held key never flashes the line.
- Flags: `--fps` (1 to 30, default 20), `--slow`, `--reduced-motion`, `--duration <minutes>`, `--seed`, `--colour`, `--headless`, `--frames` and `--size`, plus `--list-scenes`, `--help` and `--version`.
- Reduced motion from the flag, from `ZACESS_REDUCED_MOTION=1` and from the system setting on macOS, GNOME and Windows, read without extra dependencies. It caps the frame rate at 12, halves every speed, removes lightning, slows each campfire row to one change every 1.5 seconds, lengthens every pulse and keeps code rain to three rows a second with its characters never swapped. Each settings command gives up after half a second.
- Flash limits held by tests: lightning at most once every five seconds and only as a bolt, also while the terminal is being resized, no flame cell changing more than twice a second, pulses of at least 2.5 seconds and gull wing beats of 1.6 seconds.
- Resizes are acted on once the size has settled for 0.15 seconds. A scene keeps what it can, such as the settled snow and the flame, so dragging a window edge never makes the screen blink.
- A plain line for screen reader users before the scene takes the screen and one after it, plus a terminal that is restored on quit, on a panic, on Ctrl+C and on `SIGTERM`, `SIGHUP`, `SIGQUIT` or `SIGINT`. Each frame is written in one piece inside a synchronised update. Nothing is redrawn while a paused scene cannot change.
- A headless mode that prints the last frame as plain text, used by the tests and by CI on Linux, macOS and Windows for every scene in every theme at 20 by 8 and 120 by 40.
- CI on the three platforms (formatting, Clippy with warnings as errors, tests, the palette table check and the headless renders) plus a build check on Rust 1.85, markdown linting and a release workflow that builds binaries for Apple silicon and Intel Macs, static x86-64 and ARM64 Linux and 64 bit Windows (with the C runtime linked statically) and attaches them, with SHA-256 files, to the release a maintainer makes from a signed tag.
- Packaging templates for a Homebrew formula and a winget manifest, with the maintainer steps for crates.io, Homebrew and winget in `packaging/README.md`.
- The README, this changelog, the accessibility statement, the contributing guide, the security policy, the support guide, the code of conduct, a licensing notice for the Zacess mark and the palette, issue forms for a bug, a scene idea and an accessibility problem, a pull request template, CODEOWNERS and Dependabot for the actions and the Cargo dependencies.
- `docs/scenes.md`, which describes every scene and its reduced motion behaviour.

[Unreleased]: https://github.com/zacesshq/scenes/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/zacesshq/scenes/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/zacesshq/scenes/releases/tag/v0.1.0
