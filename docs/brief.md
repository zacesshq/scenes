# Zacess Scenes, first version: brief

A short decision record for the first version, written before the build and kept current when a
decision changes.

## Why

- **Who it is for:** people who use the Zacess themes and want something calm on a spare terminal,
  a second screen or a locked machine: students revising, people who find a busy screen tiring and
  anyone with low vision or motion sensitivity for whom the usual terminal screensavers are too
  fast, too dim or flash.
- **The problem today:** terminal animations are built for effect. They flicker, flash, use low
  contrast colours chosen for mood and ignore reduced motion. Nothing draws in the Zacess palette,
  so a person who has set up every editor and terminal on one theme gets a different look the
  moment they run one.
- **What success looks like:** a scene runs in all eight themes with every colour at 4.5 to 1 or
  better, nothing flashes, reduced motion is honoured from the system setting without being asked,
  the terminal always comes back and the same program works on a phone-sized terminal and a full
  screen on macOS, Linux and Windows.

## In this version

| Feature | Done when |
| --- | --- |
| Ten scenes: sea, rain, snow, starfield, code rain, fireflies, aurora, campfire, waves and the Zacess mark | Each has a headless render at 20 by 8 and 120 by 40 that looks like what it claims plus its own tests |
| The eight Zacess themes embedded from the palette, with the theme found from the flag, the variable, the switcher's state file or the default | `--list-themes` marks the active theme and the theme test passes |
| Contrast held at 4.5 to 1 for every drawn colour in every theme | `every_drawable_colour_keeps_the_contrast_floor_in_every_theme` and each scene's shared checks pass |
| Pause, help and quit keys with a help line that fades | The keys are in the README, the help text and ACCESSIBILITY.md and the overlay test passes |
| Reduced motion from the flag, the variable and the system setting on macOS, GNOME and Windows | The rain, campfire, starfield and fireflies tests prove the calm behaviour |
| Nothing flashes, nothing changes more than three times a second | Every period is a named constant with a test |
| The terminal restored on quit, panic and Ctrl+C, with a plain line before and after | Checked by hand in Terminal.app, iTerm2, Ghostty and Windows Terminal |
| Headless rendering with a seed | CI draws every scene in every theme on three platforms |
| Release binaries for five targets attached to a maintainer-made release | The first tag's workflow run attaches ten files |

## Not in this version

- Switching scene with a key while one runs or a mode that cycles through scenes. A later version;
  for now one scene runs per invocation.
- Sound. Zacess Scenes stays silent.
- A background colour of its own. The terminal is assumed to be on the same theme, which is how the
  Zacess switcher leaves it; drawing a background would fight terminals that set their own.
- Reading the reduced motion setting on desktops other than macOS, GNOME and Windows. The flag and
  the variable cover them.
- Homebrew and winget packages published. The templates ship now; publishing follows the first
  release.
- Scenes beyond the ten. Ideas go through the scene idea form.

## Data

| What is read | Where from | Written anywhere |
| --- | --- | --- |
| The theme name and the reduced motion choice | `ZACESS_THEME`, `ZACESS_REDUCED_MOTION`, `NO_COLOR`, the flags | No |
| The switcher's current theme | `$XDG_STATE_HOME/zacess/theme` or `~/.local/state/zacess/theme` | No |
| The system's reduce motion and dark mode settings | `defaults`, `gsettings` or `reg`, read only | No |

Nothing is stored, nothing is sent anywhere and no file is written. There are no secrets in the
repository; the crates.io token lives only in a maintainer's environment.

## Stack and limits

- **Built on:** Rust 2024 with crossterm as the only runtime dependency, so one static binary per
  platform. Scenes draw into a frame buffer that renders to a string, which is what makes headless
  tests and CI possible.
- **Free-tier limits it will hit:** none in use. The repository is public, so CI minutes are free.
  A release run builds five targets, about fifteen minutes of runner time.

## Accessibility

- What this version must meet: every colour at 4.5 to 1 on the background in every theme, no
  flashing, nothing changing more than three times a second, reduced motion honoured, the keys
  documented and reachable without a mouse, the terminal always restored and a plain line for
  screen readers before and after the scene.
- Specific to this product: a scene must read without colour (`NO_COLOR`), which is why kinds of
  things differ in shape and not only in colour.

## How it ships

1. One pull request with the whole first version, reviewed and squash-merged.
2. The version in `Cargo.toml` and the changelog's `Unreleased` section cut into `0.1.0`, merged by a
   maintainer.
3. A signed tag `v0.1.0` and a GitHub release made from it; the release workflow attaches the
   binaries.
4. `cargo publish`, then the Homebrew tap and the winget manifest, following `packaging/README.md`.

## Risks

- **Block characters.** The aurora, campfire, snow bank and the mark use shade and block characters
  that a few terminal fonts lack. Every other character is ASCII and the gap is documented; a
  `--ascii` fallback is a possible later addition.
- **Terminals that set their own background.** The measured contrast assumes the theme's
  background. The README says so.
- **Cross-compiling for ARM64 Linux with musl** depends on the cross toolchain action. If a target
  fails on the first release, the other four still attach and the failed one is fixed in a patch.
- **The switcher's state file format** is shared with another tool. A change there would need a
  matching change here; the parser is small and tested.

## Open questions

- Whether a later version should draw its own background from the theme behind a flag, for a
  terminal that is not on a Zacess theme. Current pick: no, keep the program honest about what it
  draws on.
- Whether code rain should offer the Latin letters only or add digits and symbols. Current pick:
  letters, digits and symbols, as shipped.
