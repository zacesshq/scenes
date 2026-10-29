# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
