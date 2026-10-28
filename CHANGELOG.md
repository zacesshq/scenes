# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Ten scenes: `sea`, `rain`, `snow`, `starfield`, `code-rain`, `fireflies`, `aurora`, `campfire`, `waves` and `zacess`. Each lays itself out for any terminal from 20 columns by 8 rows upwards and again on every resize.
- The eight Zacess themes from Zacess Themes 0.4.0, embedded from `palette/zacess.json` by `scripts/build-palette.py`. The theme comes from `--theme`, then `ZACESS_THEME`, then the Zacess theme switcher's state file (a fixed theme or `auto` with the system's dark or light mode), otherwise Zacess Deep Contrast Dark. `--list-themes` prints them and marks the active one. `NO_COLOR` draws in the terminal's own text colour.
- A contrast check in the code and in the tests: every colour a scene can draw keeps at least 4.5 to 1 on the theme's background, in every theme, at every size.
- Keys: Space pauses and resumes, `h` shows the help line again and `q`, `Esc` or `Ctrl+C` quits. The help line shows for a few seconds on start, dims and then goes; a paused scene says so.
- Flags: `--fps` (1 to 30, default 20), `--slow`, `--reduced-motion`, `--duration <minutes>`, `--seed`, `--headless`, `--frames` and `--size`, plus `--list-scenes`, `--help` and `--version`.
- Reduced motion from the flag, from `ZACESS_REDUCED_MOTION=1` and from the system setting on macOS, GNOME and Windows, read without extra dependencies. It caps the frame rate at 12, halves every speed, removes lightning, slows the campfire's shape changes to one a second, lengthens every pulse and stops code rain's characters changing.
- Flash limits held by tests: lightning at most once every five seconds and only as a bolt, the campfire's shape at most every 0.34 seconds with a smooth blend between shapes, pulses of at least 2.5 seconds and gull wing beats of 1.6 seconds.
- A plain line for screen reader users before the scene takes the screen and one after it, plus a terminal that is restored on quit, on a panic and on Ctrl+C.
- A headless mode that prints the last frame as plain text, used by the tests and by CI on Linux, macOS and Windows for every scene in every theme at 20 by 8 and 120 by 40.
- CI on the three platforms (formatting, Clippy with warnings as errors, tests, the palette table check and the headless renders), markdown linting and a release workflow that builds binaries for Apple silicon and Intel Macs, static x86-64 and ARM64 Linux and 64 bit Windows and attaches them, with SHA-256 files, to the release a maintainer makes from a signed tag.
- Packaging templates for a Homebrew formula and a winget manifest, with the owner steps for crates.io, Homebrew and winget in `packaging/README.md`.
- The README, this changelog, the accessibility statement, the contributing guide, the security policy, the support guide, the code of conduct, issue forms for a bug, a scene idea and an accessibility problem, a pull request template, CODEOWNERS and Dependabot for the actions and the Cargo dependencies.
- `docs/brief.md`, the one-page brief for this version, plus `docs/scenes.md`, which describes every scene and its reduced motion behaviour.
