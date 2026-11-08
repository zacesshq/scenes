# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0] - 2026-11-08

### Added

- Five new background sounds, ten in all, each a seamless 20 second loop made by the program. `stream` is a babbling stream with rising bubbles. `birds` is a soft breeze with phrases of birdsong. `hum` is a quiet machine room with a low hum and a fan. `city` is distant traffic with cars passing far off. `pad` is a slow warm chord that swells and fades. Scenes that suit them now play them: `forest-stream` the stream, the meadows and gardens the birds, the circuit boards the hum, the streets and pit stops the city and the calm sky and pattern scenes the pad.
- `--ambient-sound <name>`, `ZACESS_AMBIENT_SOUND` and `sound` under `[ambient]` play one background sound under every scene. `auto`, the default, keeps each scene's own.
- `zacess-scenes sounds ambient` plays a few seconds of each background sound and `sounds ambient <name>` plays one. `zacess-scenes list` names them.
- Two new events in every sound set. `clean` plays a rising fifth when a passing run has no warnings after the last one had some, answering the warnings tone. `stopped` plays a slow falling triad when the watch stops on its own because bacon quit or the followed command could not start. Both are on by default and can be turned off under `[sound.events]`.
- `Ambient::set_ambience`, `Ambient::set_volume` and `Ambient::with_fixed`, so a program built on the crate can choose the background sound and its level itself. Zacess Games uses them for the sound under each game.

## [1.0.0] - 2026-11-08

### Added

- Every built-in scene is painted in solid pixels. A renderer in `src/pixels.rs` puts two pixels in each character cell as an upper half block on a coloured background, with shades mixed from the theme's own colours in the OKLab space. Background imagery stays below the 3 to 1 non-text floor and everything that carries meaning stands out at or above it, so with `NO_COLOR` the imagery drops away and the shapes keep their outline. In 256 colours each pixel is sent as its nearest table entry.
- 26 new scenes, 65 in all: 14 general, 27 pass and 24 fail. Each has its own captions and background sound.
  - The general scenes `forest-stream`, `desert-night`, `rooftops` and `lily-pads`.
  - The pass scenes `sunflower-field`, `mountain-lake`, `tide-pools`, `paper-boats`, `windmills` and `picnic`.
  - The fail scenes `volcano`, `rockslide`, `dust-storm`, `thin-ice`, `short-circuit` and `overgrown`.
  - Five pass and fail pairs that share one picture and tell its story each way. In `drone-landing` a quadcopter maps the ground with a faint beam and settles on a marked pad; in `landing-abort` its scan finds no terrain match and it climbs away. In `kettle-boil` a kettle boils, clicks off and pours into a mug; in `kettle-cold` it clicks off cold beside a note to try again. In `rocket-launch` a rocket counts down and lifts off from a coastal pad; in `launch-scrub` it is held on the pad venting steam while the board reads HOLD. In `pit-stop` a race car has its wheels changed and leaves as the lollipop lifts; in `pit-stop-stuck` one wheel will not come off while the time on the board climbs. In `pcb-bring-up` a board on a bench lights its LEDs in turn up to POWER GOOD; in `dead-led` one LED stays dark and a probe moves to it.
- A light-theme look for many scenes, chosen from the theme rather than only recoloured: a day, a dawn or a sunrise in place of a night or a dusk, with its own sky, light and details. Each scene's section in `docs/scenes.md` says what changes.
- Zacess Warm Dark and Zacess Warm Light from Zacess Themes 0.12.1, with the short names `warm-dark` and `warm-light`.
- Lua scenes can paint solid pixels on the same two-pixels-to-a-cell canvas as the built-in scenes with `ctx.pixel`, `ctx.rect`, `ctx.disc`, `ctx.line` and `ctx.blend`, each taking a palette role and a strength from the background to the role's full colour. Text drawn over a picture keeps its full contrast or falls back to the plain background. Pixels pass the same flash meter as text. Every shape is cut to the canvas before it is painted and one frame may make at most 50,000 pixel calls that between them touch at most 64 screens' worth of pixels. Past that the rest of the frame's imagery is left out, so even a frame of screen-sized discs paints in a few milliseconds.
- Lua scenes can read the local date and time from `ctx.clock`: the hour, minute and second, the date, the weekday and month names and ready-made `time` and `date` strings. It is refreshed once a second and is `nil` when the time zone is unknown.
- `snapshot --clock` sets where a Lua scene's clock starts, as `HH:MM` or `YYYY-MM-DDTHH:MM`. Without it a snapshot's clock starts at Monday 1 June 2026 at 09:00 and moves on with `--time`, so a still is the same on every machine. Years run from 1 to 9999. A year outside that range or a day the month does not have is refused with a message that names it.
- `snapshot --state pass` and `--state fail` run a Lua scene's `on_pass` or `on_fail` hook as it starts, as the watch mode does when a run finishes. The still shows the caption the hook returns.
- One shared pixel check, `check_pixel_scene`, that every scene passes in all ten themes: sizes from one cell to 400 by 120, the flash limits at every motion setting, a still picture while paused, the imagery rules, the outline with colour off, the 256 colour fallback, text over the scene at the floor, fewer changes under reduced motion and the same pixels from the same seed.
- The Zacess Scenes mark in `assets/` in light, dark, automatic and single colour versions, with a light and a dark lockup at the head of the README. A dark and a light banner and a 1280 by 640 social preview image sit beside them.

### Changed

- CI runs the tests with a `ci` profile: release speed with overflow and debug assertion checks on, so the suite finishes in a few minutes on every platform and still catches arithmetic slips.
- The embedded palette moves to Zacess Themes 1.0.0, which gives Zacess Warm Light its final code, selection, current line and find match colours.
- The original ten scenes and the 29 earlier pass and fail scenes are repainted in pixels and many have new compositions. Among them `sea` has fish with fins and forked tails, `rain` is a lamplit wet street with a walker under an umbrella, `fireflies` is a meadow at dusk under a great tree, `zacess` floats the mark large in the middle of the screen, `storm` has rolling cloud over open country and a tree in full leaf, `koi-pond` has koi seen from above with fins and classic patterns, `mountain-lake` is a lake at first light, `leak` is a lit basement, `corrosion` is rusting riveted steel, `traffic-jam` is a side-on queue at a red signal and `fog` is a road at dusk where a wall of fog rolls in.
- A terminal colour below a theme's floor keeps its hue: it is replaced by its bright or plain twin when that keeps the floor and otherwise only its lightness moves until it reaches the floor. Before, it borrowed the nearest palette colour, which on Cream turned the yellow into the muted ink and left two roles the same colour.
- Each theme's contrast floor comes from the palette's text contrast for that theme rather than from its name: 7 to 1 on the first eight themes and 4.5 to 1 on the Warm pair. Zacess Light, Dark, Yellow on Black and Cream rise from 4.5 to 1 to 7 to 1. No theme's floor went down.
- The palette copy is Zacess Themes 0.12.1 and `scripts/build-palette.py` takes any number of themes.
- The showcase frames in `docs/frames` play six seconds at 6 frames a second, which keeps each file between about 0.7 and 1.1 MB.
- The crate, Homebrew and winget descriptions name the ten Zacess themes and the winget description the 65 scenes.
- Six scenes are redrawn to a higher standard. `forest-stream` winds out from between bushes on the horizon between mossy banks, with trunks at three depths and a bright daytime wood on light themes. `corrosion` lifts its steel so rivets and seams read on dark themes. In `volcano` the embers rise one above another up the plume, the lava keeps an orange hue on dark themes and the cone and ranges stand out of the dusk. `aurora` has a far and a near range of peaks, each with a lit and a shaded face and ragged snow caps. In `lanterns` each near and middle lantern rises in a column of its own, so two never merge into one shape. In `pit-stop` and `pit-stop-stuck` a larger car and crew fill the frame at 80 by 24 and 200 by 50, with the far side crew and the lollipop behind the car and its board clear of the wheels, a rear wing on a pylon and the tyre stacks kept clear.
- The forest light shafts in `forest-stream`, the curtains' breathing in `aurora` and how often a frog hops in `lily-pads` are held to the standard pace on lively, where they came round faster before. `ACCESSIBILITY.md` says so.
- `--help` heads the first list of scenes "General scenes, also shown while a build passes" in place of "The original scenes". The `--reduced-motion` help says the flicker is slower rather than gone.
- `ACCESSIBILITY.md`, the README, CONTRIBUTING, `docs/lua.md`, `docs/themes.md` and the winget description state the contrast rules for pixel art: text keeps the theme's floor, the pixel shapes that carry meaning keep at least 3 to 1 against the background and background imagery stays below 3 to 1.
- `docs/scenes.md` describes what each scene draws and its flashing row from the code. The gallery stills and the showcase frames are made again for every redrawn scene.
- The bug report and accessibility problem forms list the 26 new scenes and both Warm themes and their version placeholders read 0.6.0. The scene idea form asks how a scene looks on a light theme and states the contrast rules for pixel art in all ten themes.
- SUPPORT.md gives support@zacess.com for help with a scene, watch mode or follow mode and for private accessibility reports, as does the accessibility problem form, with contact@zacess.com kept for anything else.

### Fixed

- The drought scene's horizon row no longer overflows a plate index in a build with overflow checks; it wraps as the release build always has, so the picture is unchanged.
- SVG stills and JSON frames no longer show faint seams between rows. A still draws a solid shape as one rectangle over the half rows it covers and a JSON frame gives each pixel cell its top pixel as the background and its bottom pixel as a lower half block, which overlaps the line to the next row.
- The JSON frames from `snapshot --frames` carry each cell's background, so a page playing them shows both pixels of every cell.
- The watch mode's opening from a fail scene to a pass scene copies whole cells, so the scene behind keeps its backgrounds.
- The Lua instruction budget test counts instructions rather than wall-clock time, so it passes on a busy machine.

### Removed

- The generic warning triangle, from every scene that carried one. Where a scene still needs words they are on a board of their own, such as BRIDGE OUT on `broken-bridge`, KEEP BACK on `erosion` and ROAD CLOSED on `traffic-jam`; the rest show what is wrong in the picture itself.
- The `! tangled` tag in `tangle` and the pressure gauge and wet floor sign in `leak`.

## [0.6.0] - 2026-11-03

### Added

- The crate is published on crates.io, so `cargo install zacess-scenes` builds the program from the source of this release. The crate carries the code, the docs and the licence; the gallery stills, the showcase frames, the packaging files and the scripts stay out of it.
- The Homebrew formula for the `zacesshq/tap` tap and the winget manifests for `Zacess.Scenes` are filled in for this release, so the tap and the winget submission follow straight from the release files.
- A diagram in the README of how a scene is drawn: the settings, the theme, the scene, the renderer and where a frame goes.

### Changed

- The README's install section covers each route on its own: Homebrew on macOS and Linux, winget on Windows, Cargo from crates.io, the release archives for each platform with the checksum check and a script that fetches the newest release for the machine it runs on.
- Community files follow the [AI policy](https://github.com/zacesshq/.github/blob/main/AI_POLICY.md): pull requests declare AI use and issue forms ask whether AI helped. Security reports go to security@zacess.com.
- Routine tidy-up.

## [0.5.1] - 2026-11-01

### Fixed

- Follow mode on Windows no longer stops with "watch stopped: Access is denied" once a channel's log passes 256 KB. Emptying the log at the start of a follow now uses a handle opened for writing, since Windows refuses to shorten a file through an append-only one.

## [0.5.0] - 2026-10-30

### Added

- Watch mode for every language. Outside a Rust project with bacon, `zacess-scenes watch` finds the project's own check and runs it on each change: a Node project's `check`, `test`, `build` or `lint` script (with pnpm, yarn or bun when their lockfile is there), pytest or a compile of every Python file, `go build` and `go vet`, Maven, Gradle, `dotnet build`, `swift build`, `mix compile`, `composer test`, `rake`, `make` or a CMake build. `--cmd` runs any other command. Its exit code decides pass or fail and the lines that read as errors are shown, with their file and line where given.
- Follow mode. `zacess-scenes follow` shows how each command in a shell ended, in any language and with any tool, through a one-line hook for zsh, bash, fish and PowerShell printed by `zacess-scenes follow init <shell>`. A scene follows the shells in its own tmux session unless a channel is named with `ZACESS_FOLLOW` or `--channel`. A command that was not found shows TYPO with a soft tone of its own and the next command that works plays the recovered tone. Commands stopped with `Ctrl+C` are left out and `min_seconds` under `[follow]` can leave out quick ones.
- A slideshow for a scene run on its own. `--shuffle` moves to another scene at random every five minutes and `--every <minutes>` sets the time. With `rotation` and `interval` in the config, a scene run with no name does the same through the `pass` list.
- Background sound. `--ambient` or `enabled = true` under `[ambient]` plays a quiet loop that suits the scene: water, rain, wind, fire or a night meadow. It plays at 20 out of 100 by default, steps aside while a tone plays, pauses with the scene and keeps to quiet hours. `a` turns it on or off and `[` and `]` change its volume.
- The `typo` sound event, on by default.

### Changed

- The watch mode's failure banner reads FAILED for any command other than bacon, which keeps BUILD FAILED. The warning count shows only with bacon, the one source that reports warnings.
- A release now tells zacess.com to rebuild. Once every platform's files are attached, the release workflow sends a `scenes-release` repository dispatch naming the tag to `zacesshq/website`, so the site no longer waits for its daily deploy.

### Fixed

- Two of the watch mode's key hints were one or two columns wider than the narrowest terminal they were shown on and were cut short there.

## [0.4.0] - 2026-10-29

### Added

- Two sound sets, `piano` and `rain`, so there are eight. The piano has a felt hammer strike and a warm low ring, its overtones a little sharp of whole multiples as a stiff string's are. The rain set plays each note as a water droplet whose pitch rises as it rings, over a soft bed of filtered rain. Both play the same phrase for each event as the other sets and are levelled to the same loudness.

### Changed

- The README is shorter: the gallery, a quick start, the sound sets, install and accessibility, with a table of documentation pages. The detail moved into `docs/usage.md` (commands, keys, flags and headless rendering), `docs/watch-mode.md` (with notifications), `docs/sounds.md`, `docs/configuration.md`, `docs/themes.md` and `docs/snapshots.md`. The scene tables are at the top of `docs/scenes.md`.

### Fixed

- The README installs with Cargo from the release tag on GitHub, since the crate is not on crates.io yet. It also lists mlua and the C compiler that building Lua needs.

## [0.3.0] - 2026-10-29

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

[Unreleased]: https://github.com/zacesshq/scenes/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/zacesshq/scenes/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/zacesshq/scenes/compare/v0.6.0...v1.0.0
[0.6.0]: https://github.com/zacesshq/scenes/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/zacesshq/scenes/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/zacesshq/scenes/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/zacesshq/scenes/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/zacesshq/scenes/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/zacesshq/scenes/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/zacesshq/scenes/releases/tag/v0.1.0
