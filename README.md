# Zacess Scenes

Calm animated scenes for the terminal, drawn in the eight [Zacess](https://zacess.com) high contrast
themes.

`zacess-scenes` is one small program. It draws a scene with text characters in the colours of the
Zacess theme the terminal is using, keeps every colour at 4.5 to 1 contrast or better on the
background (7 to 1 on the High Contrast and Deep Contrast themes), never flashes and slows down when
the system or the person asks for reduced motion.
A scene fits any terminal from 20 columns by 8 rows up to a full screen and follows a resize while
it runs. The [watch mode](#watch-mode) turns the scenes into a build monitor for Rust projects.

> [!NOTE]
> This is the first version. The Homebrew tap and the winget package are being set up; until then
> install with Cargo or from a release binary.

## Scenes

| Scene | What it shows | Under reduced motion |
| --- | --- | --- |
| `sea` | Fish of several sizes, bubbles, seaweed, a boat on the surface and, now and then, a whale crossing the deep water | Everything swims at half speed and fewer bubbles rise |
| `rain` | Drops at two depths, ripples where they land and a distant bolt of lightning every few seconds at most | No lightning at all, slower rain |
| `snow` | Flakes at two depths drifting down and settling into a bank along the bottom | Slower snow |
| `starfield` | Stars that brighten and fade slowly, a crescent moon and, now and then, a slow shooting star | Stars take twice as long to change, shooting stars are rarer and slower |
| `code-rain` | Columns of characters falling in the theme's syntax colours, each with a bright head | Columns fall at most three rows a second and their characters are never swapped |
| `fireflies` | A dark meadow with lights that wander and pulse slowly | Slower, longer pulses and slower wandering |
| `aurora` | Slow waves of colour across the top of a starry sky over a mountain ridge | The waves move at half speed |
| `campfire` | Flames whose rows take a new shape one at a time, each at most once every 0.6 seconds, embers that rise and a steady glow on the ground | Each row changes at most once every 1.5 seconds, fewer and slower embers |
| `waves` | A calm sea to the horizon under a sun, with gulls crossing the sky | Slower swell and slower wing beats |
| `zacess` | The Zacess mark drifting slowly and bouncing gently off the edges | It drifts at half speed |

Every scene is described in more detail, with its colours and its tests, in
[docs/scenes.md](docs/scenes.md).

## Install

### With Cargo

```sh
cargo install zacess-scenes
```

Rust 1.85 or later. The only dependency is crossterm, plus signal-hook on Unix, which crossterm
already uses.

### From a release

Each [release](https://github.com/zacesshq/scenes/releases/latest) carries a built binary for:

| File | System |
| --- | --- |
| `zacess-scenes-<tag>-aarch64-apple-darwin.tar.gz` | macOS on Apple silicon |
| `zacess-scenes-<tag>-x86_64-apple-darwin.tar.gz` | macOS on Intel |
| `zacess-scenes-<tag>-x86_64-unknown-linux-musl.tar.gz` | Linux on x86-64, no shared libraries needed |
| `zacess-scenes-<tag>-aarch64-unknown-linux-musl.tar.gz` | Linux on ARM64, no shared libraries needed |
| `zacess-scenes-<tag>-x86_64-pc-windows-msvc.zip` | Windows on x86-64 |

Each archive has a `.sha256` file beside it. Check the sum, unpack the archive and put
`zacess-scenes` (or `zacess-scenes.exe`) somewhere on your `PATH`.

```sh
shasum -a 256 -c zacess-scenes-v0.1.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf zacess-scenes-v0.1.0-aarch64-apple-darwin.tar.gz
```

### Homebrew and winget

Coming. The formula and the manifests are in [packaging](packaging) and will be published once the
first release exists.

## Use

```sh
zacess-scenes                       # the sea, in the active theme
zacess-scenes campfire              # one scene by name
zacess-scenes snow --theme cream    # in a named theme
zacess-scenes aurora --duration 10  # stops on its own after ten minutes
zacess-scenes --list-scenes
zacess-scenes --list-themes
```

### Keys

| Key | Does |
| --- | --- |
| `Space` | Pauses and resumes |
| `h` or `?` | Shows the help line again |
| `q`, `Esc` or `Ctrl+C` | Quits and gives the terminal back |

The help line shows for a few seconds when a scene starts and then fades. While a scene is paused
the line says so.

### Flags

| Flag | Does |
| --- | --- |
| `--theme <name>` | A Zacess theme by id, short name or full name, such as `deep-contrast-dark`, `deep-dark` or `Zacess Deep Contrast Dark` |
| `--list-themes` | Prints the themes and marks the one that would be used |
| `--list-scenes` | Prints the scenes |
| `--fps <n>` | Frames per second, 1 to 30. The default is 20 |
| `--slow` | Half speed |
| `--reduced-motion` | Fewer and slower movements, no lightning and no flicker, at most 12 frames a second |
| `--duration <minutes>` | Stops after this many minutes, decimals allowed |
| `--colour <depth>` | `truecolor` or `256`, when the colour depth is detected wrongly. `--color` also works |
| `--seed <n>` | Seeds the random numbers, so a run can be repeated exactly |
| `--headless` | Draws without a terminal and prints the last frame as plain text |
| `--frames <n>` | How many frames to draw headless. The default is 60 |
| `--size <WxH>` | Columns and rows for a headless render, such as `20x8` |
| `-h`, `--help` | The help |
| `-V`, `--version` | The version and the palette version |

## Watch mode

`zacess-scenes watch` shows how a Rust project's build stands. It runs
[bacon](https://dystroy.org/bacon/) in the background and reads the report bacon writes after each
run:

- **Passing:** the calm `waves` scene with a large ALL GOOD banner, a check mark and a short
  caption. Any warnings are listed below it.
- **Failing:** a storm (the `rain` scene with heavier rain, wind and a second row of cloud) with a
  large BUILD FAILED banner, a cross and the first errors or failing tests, each with its location.
  Warnings follow the errors.
- **Warnings** are marked `!` and drawn in the theme's warning colour. Errors and failing tests are
  marked with a cross and drawn in the theme's error colour, so the two differ in shape as well as
  colour.

### Install bacon

bacon is a separate program, not part of Zacess Scenes (see [NOTICE.md](NOTICE.md)). Install it
once:

```sh
cargo install --locked bacon
```

Without bacon the watch mode prints that command and stops. Outside a Cargo project it says so and
stops.

### Use watch mode

```sh
cd my-project
zacess-scenes watch                   # cargo check
zacess-scenes watch clippy            # cargo clippy
zacess-scenes watch test --volume 30  # cargo test with quieter tones
zacess-scenes watch --no-sound        # no tones at all
```

| Job | Runs |
| --- | --- |
| `check` (the default) | `cargo check` |
| `check-all` | `cargo check --all-targets` |
| `clippy` | `cargo clippy` |
| `test` | `cargo test` |

bacon reruns the job whenever a file in the project changes. Its JSON report export is turned on
through bacon's `--config-toml` option and written to a temporary folder that is removed on quit.
The project's own `bacon.toml` is read as usual and never written to.

The bottom row is a status bar in the theme's colours:

```text
my-project │ check │ ✓ passing · 2 warnings │ 12s ago
```

While the build fails it reads `✗ failing` in the theme's error colour with the count of errors or
failed tests. While bacon is running the job it adds `· running`. Notes such as `paused`,
`sound off` and `no sound player` follow at the end. On a narrow terminal the notes go first, then
the age and then the job.

`--theme`, `--fps`, `--reduced-motion` and `--colour` work as they do for the scenes. Under reduced
motion the storm has no lightning.

### Keys in watch mode

| Key | Does |
| --- | --- |
| `1` to `4` | Switches to the `check`, `check-all`, `clippy` or `test` job |
| `r` | Runs the job again |
| `p` or `Space` | Pauses and resumes the animation. The status keeps updating |
| `s` | Turns sound on or off |
| `h` or `?` | Shows the keys again |
| `q`, `Esc` or `Ctrl+C` | Quits, stops bacon and gives the terminal back |

### Sounds

A gentle rising chime plays when a run passes and a lower two-note tone when it fails. Each plays
once when the result changes or when `r` or a job key asked for the run, so saving a file that still
fails stays quiet. A soft tick marks the start of each rebuild.

The tones are synthesised by the program, with soft bell envelopes and short fades so they never
click. They are cached as small WAV files in `$XDG_CACHE_HOME/zacess-scenes` when that is set,
otherwise `~/Library/Caches/zacess-scenes` on macOS, `~/.cache/zacess-scenes` on Linux and
`%LOCALAPPDATA%\zacess-scenes` on Windows. The system's own player plays them in the background:
`afplay` on macOS, the first of `pw-play`, `paplay` or `aplay` on Linux and PowerShell's
`System.Media.SoundPlayer` on Windows. Without a player there is no sound and the status bar says
`no sound player`.

| Setting | Flag | Environment variable |
| --- | --- | --- |
| Sound off | `--no-sound` | `ZACESS_SOUND=0` (also `false`, `no` or `off`) |
| Volume from 0 to 100, 50 by default | `--volume <n>` | `ZACESS_VOLUME=<n>` |

The flag wins over the variable and a volume of 0 is silent. Every tone has the same state on
screen, so sound is never the only signal.

## Themes

The colours are the terminal and syntax colours of [Zacess
Themes](https://github.com/zacesshq/themes)
0.5.0, embedded in the program. The theme is picked in this order:

1. `--theme`
2. the `ZACESS_THEME` environment variable
3. a theme state file, `$XDG_STATE_HOME/zacess/theme` or `~/.local/state/zacess/theme`, which any
   tool that sets the terminal's theme can write. It holds a theme id on its own, such as
   `deep-contrast-dark`. It can instead hold `auto` followed by a dark and a light id, such as
   `auto deep-contrast-dark deep-contrast-light`. With `auto` the dark one is used when the system
   is in dark mode and the light one otherwise. The program only reads the file
4. Zacess Deep Contrast Dark

| Id | Short name | Theme |
| --- | --- | --- |
| `light` | `light` | Zacess Light |
| `dark` | `dark` | Zacess Dark |
| `yellow-on-black` | `yellow` | Zacess Yellow on Black |
| `cream` | `cream` | Zacess Cream |
| `high-contrast-dark` | `hc-dark` | Zacess High Contrast Dark |
| `high-contrast-light` | `hc-light` | Zacess High Contrast Light |
| `deep-contrast-dark` | `deep-dark` | Zacess Deep Contrast Dark |
| `deep-contrast-light` | `deep-light` | Zacess Deep Contrast Light |

A name is matched without regard to case, the `Zacess` prefix or whether spaces or hyphens
separate the words. The program sets no background colour: it draws on whatever the terminal
already shows, so the terminal should be on the same theme.

### Colour depth

Colours are sent as exact 24 bit colour when the terminal says it supports it (`COLORTERM` set to
`truecolor` or `24bit`, Windows Terminal, iTerm2, WezTerm, Ghostty and a few others). Anywhere
else, such as Terminal.app before macOS 26, the Linux console or tmux without true colour, each
colour is sent as the nearest entry in the 256 colour table that still keeps the contrast floor.
`--colour truecolor` or `--colour 256` overrides the guess.

With the `NO_COLOR` environment variable set, every character is drawn in the terminal's own text
colour and the scene is still readable, because no scene relies on colour alone.

## Accessibility

Nothing flashes. Things that change in place, such as the flame, a star's brightness and the
lightning, keep to measured limits, the fastest being a flame cell at most twice a second; things
that move change the cells they pass through. Reduced motion is honoured from `--reduced-motion`,
from `ZACESS_REDUCED_MOTION=1` and from the system setting on macOS, GNOME and Windows. A plain
line is printed before the scene takes over the screen, so a screen reader user hears what started
and how to quit. The terminal is given back after a quit, a panic, Ctrl+C or a termination
signal. In the watch mode every tone is paired with the same state on screen and sound can be
turned off or made quieter. Every colour is checked against its background in the tests. The full statement,
with the measured limits, is in [ACCESSIBILITY.md](ACCESSIBILITY.md).

## Headless rendering

`--headless` draws a scene without a terminal and prints the last frame, which is how the tests and
CI look at every scene in every theme on Linux, macOS and Windows:

```sh
zacess-scenes sea --headless --frames 60 --size 20x8 --seed 1 --theme deep-dark
```

The same seed, size, frame count and theme always print the same frame.

> [!TIP]
> Windows PowerShell 5.1 decodes a program's output with the console's code page, so a saved
> frame shows the block characters as other characters. Run
> `[Console]::OutputEncoding = [Text.Encoding]::UTF8` first or use PowerShell 7.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). A new scene implements one trait and is checked by the same
tests as the rest.

## Licence

[MIT](LICENSE). The licence does not cover the Zacess name or mark; see [NOTICE.md](NOTICE.md).
