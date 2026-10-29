# Zacess Scenes

Calm animated scenes for the terminal, drawn in the eight [Zacess](https://zacess.com) high contrast
themes. It is also a build monitor for Rust that shows how a build stands as a scene.

| While the build passes | While it fails |
| --- | --- |
| ![The watch mode while a build passes: a calm sea under a sun with a large ALL GOOD banner, a check mark, a caption and a status bar reading passing](docs/images/scenes/watch-pass.svg) | ![The watch mode while a build fails: a storm with a large BUILD FAILED banner, a cross, two errors with their locations and a status bar reading failing](docs/images/scenes/watch-fail.svg) |

`zacess-scenes` is one small program. It draws a scene with text characters in the colours of the
Zacess theme the terminal is using, keeps every colour at 4.5 to 1 contrast or better on the
background (7 to 1 on the High Contrast and Deep Contrast themes), never flashes and slows down when
the system or the person asks for reduced motion.
A scene fits any terminal from 20 columns by 8 rows up to a full screen and follows a resize while
it runs. The [watch mode](#watch-mode) turns the scenes into a build monitor for Rust projects, with
[sound sets](#sound-sets), [desktop notifications](#notifications) and a [config
file](#configuration). You can also [write your own scene in Lua](#write-your-own-scene-in-lua).

> [!NOTE]
> The Homebrew tap and the winget package are being set up; until then install with Cargo or from
> a release binary.

## Scenes

There are 39 scenes: the original ten, 16 calm pass scenes and 13 fail scenes. Every one of them runs
on its own. In the watch mode the original and pass scenes show while a build passes and the fail
scenes while it fails. `zacess-scenes list` prints them all, marked pass, fail or general. See the
[gallery](#gallery) for a still of each and [docs/scenes.md](docs/scenes.md) for every scene in
detail, with its colours and its tests.

### The original scenes

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

### Pass scenes

Calm scenes made for a passing build: nature, the city, space, water, the seasons, cosy rooms and
two abstract pieces in the Zacess style.

| Scene | What it shows | Under reduced motion |
| --- | --- | --- |
| `koi-pond` | A garden pond from above, with koi gliding on slow curves under lily pads, drifting flecks and a slow ripple now and then | Koi swim at half speed with still tails, fewer and slower ripples |
| `harbour` | A harbour town at night, with lamps on the quay, their long shimmering reflections, moored boats swaying at their masts, a lighthouse and a passing ferry | Masts stay still, the shimmer changes half as often, the ferry is slower |
| `lanterns` | Paper lanterns rising slowly from a dark lake at three depths, each with a reflection that slides down the water as it climbs | Lanterns rise, sway and drift at half speed |
| `balloons` | Hot air balloons in three sizes lifting from behind three ridges of rolling hills under drifting clouds | Balloons rise and drift at half speed |
| `countryside` | A steam train crossing open country, with hills, hedgerows, trees, the track and a fence sliding past at their own speeds | Every layer slides at half speed and smoke puffs half as often |
| `winter-cabin` | A log cabin in the snow at night, with a lit window, chimney smoke, pines and gently falling snow | Snow and smoke move at half speed |
| `autumn` | A tree in autumn colour dropping leaves that turn as they drift down and gather in a pile, with geese crossing now and then | Leaves fall at half the rate without turning |
| `blossom` | A flowering branch letting petals go on a breeze, down to a stream that carries them away, with a bird on the branch | Petals fall and float at half speed without turning, the bird looks about half as often |
| `city-night` | A night skyline where windows light and darken one at a time, a moon, a river and a train that crosses a bridge now and then | Windows change half as often and the train is slower and rarer |
| `orbit` | A banded, shaded planet circled by a ring that passes behind it and in front of it, with a moon on a slow 40 second orbit | The moon takes 80 seconds to go round |
| `constellations` | Stars joined slowly into named figures of our own, one group at a time, held and then faded | Lines are drawn at half speed and fade more slowly |
| `tea` | A mug of tea with curling steam, books, a plant and a lamp, with rain or snow at the window | Fewer, slower puffs of steam and slower weather |
| `cat-nap` | A cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window | Breaths, the tail and the z's take twice as long |
| `garden` | Seeds that sprout, grow and bloom over a season of about two minutes, with butterflies, before the bed starts again | The season takes twice as long and wings beat half as often |
| `tiles` | A grid of small pages with the Zacess fold, one tile changing colour at a time | Tiles change half as often |
| `ribbons` | Ribbons of contour lines flowing slowly across a faint grid of dots | The ribbons flow at half speed |

### Fail scenes

Slow, serious scenes that read as something gone wrong without ever flashing, shaking or spinning.
Each one progresses for about a minute, then holds or eases into a gentle loop, so it still looks
right after an hour.

| Scene | What it shows | Under reduced motion |
| --- | --- | --- |
| `storm` | Twice the rain of `rain`, slanting in a strong wind under two rows of cloud, with a bolt of lightning every 5 to 9 seconds at most and never in the first five | No lightning at all, slower rain |
| `flood` | A street where the water rises slowly past the doors and holds with a gentle swell, debris drifting along the surface and a warning sign going under | The water rises, swells and drifts at half speed and the drizzle is halved |
| `listing-ship` | A cargo ship on a grey sea that settles by the bow over a minute as its waterline creeps up past the portholes, then rolls gently while a lifebuoy drifts away | It lists, rolls and drifts at half speed |
| `corrosion` | Riveted steel plates where rust creeps out from the rivets cell by cell to half the wall, darkening as it ages, with slow drips | Rust spreads and drips at half speed and each stage takes twice as long |
| `tangle` | Cables hanging from a patch panel that slowly wander, cross and knot into a swaying snarl with a warning tag | The cables tangle and sway at half speed |
| `wilt` | Potted plants on a sunny sill whose stems droop over, leaves dry and fall onto the sill and soil cracks | The plants wilt, sway and drop leaves at half speed |
| `fog` | A road with signs, lamps and pines where a fog bank rolls in and the signs fade from view, glimpsed through gaps | The fog rolls in and drifts at half speed |
| `broken-bridge` | A bridge over a flowing river with its middle span gone, girders hanging, the fallen span in the water, a barrier, a warning sign and a waiting car | The river and clouds move at half speed and fragments fall half as often |
| `leak` | A cracked pipe dripping into a bucket that fills over a minute and then overflows into a puddle that spreads and holds, with a sinking pressure gauge and a wet floor sign | Drips come half as often and fall at half speed |
| `blackout` | A night skyline whose lit windows go dark one at a time beside a snapped power line, leaving a few candles and windows on their own power | The windows go dark at half the rate and the clouds drift at half speed |
| `traffic-jam` | Cars queued bumper to bumper before a stop line, cones and a signal stuck on red, with exhaust puffs drifting up | Cars roll in at half speed and puffs come half as often |
| `erosion` | A cliff edge crumbling into the sea, stones tumbling and splashing and a fence left hanging over the drop until the face reaches harder rock | Pieces break away half as often and fall at half speed |
| `drought` | Cracked earth spreading edge by edge under a heavy sun, a dry riverbed whose last puddle shrinks away, a withered tree and drifting dust | The cracks spread and the puddle dries at half the rate |

## Install

### With Cargo

Zacess Scenes is not on crates.io yet, so Cargo installs it from a release tag here:

```sh
cargo install --locked --git https://github.com/zacesshq/scenes --tag v0.3.0 zacess-scenes
```

Rust 1.85 or later and a C compiler, since Lua 5.4 is built into the program. The dependencies are
crossterm, mlua with its own copy of Lua and signal-hook on Unix.

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
shasum -a 256 -c zacess-scenes-v0.3.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf zacess-scenes-v0.3.0-aarch64-apple-darwin.tar.gz
```

### Homebrew and winget

Coming. The formula and the manifests are in [packaging](packaging) and will be published once the
first release exists.

## Use

```sh
zacess-scenes                       # the sea, in the active theme
zacess-scenes campfire              # one scene by name
zacess-scenes lanterns --motion calm
zacess-scenes snow --theme cream    # in a named theme
zacess-scenes aurora --duration 10  # stops on its own after ten minutes
zacess-scenes list                  # every scene and sound set
zacess-scenes --list-themes
```

### Keys

| Key | Does |
| --- | --- |
| `Space` | Pauses and resumes |
| `n` | Shows the next scene and names it on the bottom row |
| `b` or `N` | Shows the scene before |
| `h` or `?` | Shows the help line again |
| `q`, `Esc` or `Ctrl+C` | Quits and gives the terminal back |

`n` and `N` do different things: the lowercase key moves on and the uppercase one goes back. The
help line shows for a few seconds when a scene starts and then fades. While a scene is paused the
line says so.

### Commands

| Command | Does |
| --- | --- |
| `zacess-scenes watch [job]` | Shows the state of a Rust build as a scene. See [watch mode](#watch-mode) |
| `zacess-scenes list` | Prints every scene (marked pass, fail or general) and the sound sets |
| `zacess-scenes config init` | Writes a fully commented config file. Add `--force` to replace one |
| `zacess-scenes config path` | Prints where the config file is |
| `zacess-scenes sounds play <set> [event]` | Plays a sound set's tone for an event, `pass` by default |
| `zacess-scenes scene new <name>` | Writes a commented starter Lua scene into the scenes folder |
| `zacess-scenes snapshot <scene>` | Draws a scene into an SVG still or JSON frames. See [snapshots](#snapshots) |

### Flags

| Flag | Does |
| --- | --- |
| `--theme <name>` | A Zacess theme by id, short name or full name, such as `deep-contrast-dark`, `deep-dark` or `Zacess Deep Contrast Dark` |
| `--scene <name>` | The scene, as an alternative to naming it first |
| `--motion <level>` | `calm`, `standard` or `lively`. Lively is faster and busier but never changes anything in place faster than standard |
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

- **Passing:** a calm scene, `waves` unless you choose others, with a large ALL GOOD banner, a check
  mark and a short caption that belongs to the scene. Any warnings are listed below it.
- **Failing:** a fail scene, `storm` unless you choose others, with a large BUILD FAILED banner, a
  cross, a caption and the first errors or failing tests, each with its location. Warnings follow
  the errors.
- **Recovered:** when a failing build is fixed, the pass scene opens out from the middle of the
  screen over the fail scene in two seconds, its edge traced by a ring of small sparkles. The status
  bar reads `recovered` and a recovered caption shows for a few seconds. Under reduced motion the
  opening takes three and a half seconds and has no sparkles. Nothing flashes.
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
zacess-scenes watch --scene koi-pond,tea --fail-scene flood --sounds marimba --notify
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

`--theme`, `--fps`, `--motion`, `--reduced-motion` and `--colour` work as they do for the scenes.
Under reduced motion the storm has no lightning. A change of scene, sound set or volume is written
in the status bar for three seconds, such as `scene koi-pond · sounds marimba`. The status bar
shows `quiet hours` while [quiet hours](#configuration) keep the sound off.

### Choosing the scenes

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

### Keys in watch mode

| Key | Does |
| --- | --- |
| `1` to `4` | Switches to the `check`, `check-all`, `clippy` or `test` job |
| `r` | Runs the job again |
| `n` | Shows the next scene for the current state |
| `b` or `N` | Shows the scene before |
| `m` | Moves to the next sound set and plays its pass tone |
| `+` and `-` | Raise and lower the volume by 10. `=` and `_` work too, so Shift is not needed |
| `s` | Turns sound on or off |
| `p` or `Space` | Pauses and resumes the animation. The status keeps updating |
| `h` or `?` | Shows the keys again |
| `q`, `Esc` or `Ctrl+C` | Quits, stops bacon and gives the terminal back |

A scene changes at most once every 0.4 seconds however fast `n` or `b` is pressed, so the picture
is never replaced faster than three times a second.

## Sound sets

Every set plays the same short phrase for each event, so the meaning carries over when the set
changes: a rising triad for a pass, a falling fourth for a failure, the triad landing an octave up
for a fix and a rising minor third for new warnings. Only the timbre differs.

| Set | Sounds like |
| --- | --- |
| `bells` (the default) | Soft bells with a little shimmer, the original tones |
| `marimba` | Warm wooden bars, short and round |
| `kalimba` | Plucked metal tines with a gentle ring |
| `pad` | A soft synth pad that swells in and fades out |
| `chip` | Gentle 8-bit square waves, rounded so they never buzz |
| `glass` | Slow glassy tones like a wet finger on a rim |

| Event | Plays when | On by default |
| --- | --- | --- |
| `pass` | A run passes after waiting or a key asked for the run | Yes |
| `fail` | A run fails after passing, on the first result or when a key asked for the run | Yes |
| `tick` | A rebuild starts. Quieter than the others | Yes |
| `recovered` | A run passes after failing | Yes |
| `warnings` | A passing run has more warnings than the last | Yes |
| `still_failing` | A run fails again. Very quiet | No |

```sh
zacess-scenes sounds play marimba            # the pass tone
zacess-scenes sounds play kalimba recovered  # one event
```

The tones are synthesised by the program, with soft envelopes and short fades so they never click
and levelled so no set or event is much louder than another. They are cached as small WAV files in
`$XDG_CACHE_HOME/zacess-scenes` when that is set, otherwise `~/Library/Caches/zacess-scenes` on
macOS, `~/.cache/zacess-scenes` on Linux and `%LOCALAPPDATA%\zacess-scenes` on Windows. The
system's own player plays them in the background: `afplay` on macOS, the first of `pw-play`,
`paplay` or `aplay` on Linux and PowerShell's `System.Media.SoundPlayer` on Windows. Without a
player there is no sound and the status bar says `no sound player`.

| Setting | Flag | Environment variable | Config |
| --- | --- | --- | --- |
| Sound off | `--no-sound` | `ZACESS_SOUND=0` (also `false`, `no` or `off`) | `enabled = false` under `[sound]` |
| The sound set | `--sounds <set>` | `ZACESS_SOUNDS=<set>` | `set` |
| Volume from 0 to 100, 50 by default | `--volume <n>` | `ZACESS_VOLUME=<n>` | `volume` |
| Quiet hours | | | `quiet_hours = "22:00-07:00"` |
| One event on or off | | | `[sound.events]` |

Every tone has the same state on screen, so sound is never the only signal.

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

## Configuration

Every setting can live in one file:

| System | File |
| --- | --- |
| macOS and Linux | `$XDG_CONFIG_HOME/zacess/scenes.toml` when `XDG_CONFIG_HOME` is set, otherwise `~/.config/zacess/scenes.toml` |
| Windows | `%APPDATA%\zacess\scenes.toml` |

`ZACESS_CONFIG` points at another file. `zacess-scenes config path` prints where it is and
`zacess-scenes config init` writes a starter with every setting commented out at its default. A
flag wins over an environment variable, which wins over the file, which wins over the default.
Reduced motion is the exception: any of them (or the system setting) turns it on and none of them
turns the system setting off. A setting the file does not know or a value it cannot use is an
error that names the line.

A full example:

```toml
[scenes]
theme = "deep-dark"
pass = ["waves", "koi-pond", "lanterns", "city-night", "my-lantern"]
fail = ["storm", "flood", "fog"]
rotation = "cycle"      # fixed, cycle or shuffle
interval = 15           # minutes; 0 changes only on a new result
motion = "standard"     # calm, standard or lively
reduced_motion = false
captions = true
banner = true           # off draws ALL GOOD or BUILD FAILED on one line instead

[sound]
enabled = true
set = "kalimba"         # bells, marimba, kalimba, pad, chip or glass
volume = 40
quiet_hours = "22:00-07:00"

[sound.events]
pass = true
fail = true
tick = false
recovered = true
still_failing = false
warnings = true

[notifications]
enabled = true
```

The file is read by a small parser of our own for the part of TOML it needs: tables, keys,
strings, numbers, booleans and lists, with comments. That keeps the program to the dependencies it
already had plus Lua.

## Write your own scene in Lua

A scene of your own is one `.lua` file in the `scenes` folder beside the config file
(`~/.config/zacess/scenes` on macOS and Linux, `%APPDATA%\zacess\scenes` on Windows). Lua 5.4 is
built into the program, so there is nothing to install.

```sh
zacess-scenes scene new pond   # writes a commented starter, pond.lua
zacess-scenes pond             # runs it
zacess-scenes watch --scene pond
```

A scene returns a table with a name, a kind (`pass`, `fail` or `general`), a summary, captions and
a `frame` function, plus optional `init` and `resize` hooks and `on_pass`, `on_fail` and
`on_recovered` hooks that can return a caption:

```lua
return {
  name = "pulse",
  kind = "pass",
  summary = "A single dot drifting across the screen",
  captions = { "All good.", "Nothing to fix." },
  frame = function(ctx)
    local x = math.floor((ctx.time * 4 * ctx.speed) % ctx.width)
    put(x, math.floor(ctx.height / 2), "o", "accent")
  end,
  on_recovered = function(ctx) return "Fixed. Nicely done." end,
}
```

Each frame gets the width, the height, the time, the time since the last frame, the motion level,
whether motion is reduced and a seeded random function. `put(x, y, text, role)` draws, where the
role is a palette role such as `text`, `muted`, `accent`, `error`, `warning` or `syntax.keyword`,
never a raw colour, so every user scene keeps the contrast floors automatically.

Scripts run in a sandbox: no `io`, `os`, `require`, `load`, `dofile` or `debug`, no file or network
access, a budget of instructions per frame and a memory limit, so a runaway script cannot freeze the
terminal. While a scene runs, its frames go through the same flash meter as the built-in scenes: a
frame that would change a large area more than three times a second is held back and a scene that
keeps trying is stopped. An error of any kind shows in the panel and the status bar and a built-in
scene takes over. [examples/scenes/lantern.lua](examples/scenes/lantern.lua) is a worked example and
[docs/lua.md](docs/lua.md) is the full reference.

## Snapshots

`zacess-scenes snapshot` draws a scene with the real renderer and no terminal:

```sh
zacess-scenes snapshot koi-pond --out koi-pond.svg                  # an SVG still
zacess-scenes snapshot storm --state fail --out watch-fail.svg      # the watch mode, sample data
zacess-scenes snapshot lanterns --frames 72 --fps 12 --out a.json   # JSON frames
```

A still is a self-contained SVG: the scene as a grid of monospace text in each cell's palette
colour on the theme's background, inside a rounded terminal window with the scene and theme as its
title. It has `role="img"` and an `aria-label` describing the scene. Runs of one colour are merged,
so a still is about 15 to 35 KB. `--theme`, `--size` (80x24 by default), `--time` (how many seconds
in, 6 by default), `--seed`, `--motion` and `--reduced-motion` set what is drawn. `--state pass` or
`--state fail` adds the watch mode's banner, caption and status bar with sample data. Lua scenes
work too.

With `--frames <n>` the output is JSON instead, for a web page to play in a canvas or a `pre`
element:

```json
{"width":80,"height":24,"fps":12,"background":"#000000","frames":[[{"x":3,"y":0,"text":"~~ ~","colour":"#5fd7ff"}]]}
```

Each frame is a list of runs of one colour. The frames go through the flash meter, so a page
playing them never flashes. [docs/frames](docs/frames) holds frames for eight scenes (`koi-pond`, `lanterns`, `tea`, `constellations`, `fog`, `traffic-jam`, `drought` and `zacess`), six seconds
each at 12 frames a second.

## Gallery

Every scene in Zacess Deep Contrast Dark at 80 by 24, six seconds in. `scripts/snapshots.sh`
makes them all again and CI fails when a still is out of date. [docs/scenes.md](docs/scenes.md)
describes each one.

### The original scenes

| | | |
| --- | --- | --- |
| ![The sea scene: Fish of several sizes, bubbles, seaweed, a boat on the surface and an occasional whale](docs/images/scenes/sea.svg)<br>`sea` | ![The rain scene: Drops and puddle ripples, with distant lightning unless motion is reduced](docs/images/scenes/rain.svg)<br>`rain` | ![The snow scene: Drifting flakes that settle at the bottom](docs/images/scenes/snow.svg)<br>`snow` |
| ![The starfield scene: Twinkling stars, a crescent moon and a slow shooting star](docs/images/scenes/starfield.svg)<br>`starfield` | ![The code-rain scene: Falling columns of characters in the theme's syntax colours](docs/images/scenes/code-rain.svg)<br>`code-rain` | ![The fireflies scene: A dark meadow with slow pulsing lights](docs/images/scenes/fireflies.svg)<br>`fireflies` |
| ![The aurora scene: Slow waves of colour across the top of the sky](docs/images/scenes/aurora.svg)<br>`aurora` | ![The campfire scene: Flames and embers with a steady glow](docs/images/scenes/campfire.svg)<br>`campfire` | ![The waves scene: A calm sea horizon with gulls](docs/images/scenes/waves.svg)<br>`waves` |
| ![The zacess scene: The Zacess mark drifting slowly](docs/images/scenes/zacess.svg)<br>`zacess` |   |   |

### Pass scenes

| | | |
| --- | --- | --- |
| ![The koi-pond scene: A garden pond from above, with koi gliding under lily pads and slow ripples](docs/images/scenes/koi-pond.svg)<br>`koi-pond` | ![The harbour scene: A harbour town at night, with moored boats, quay lamps and their reflections](docs/images/scenes/harbour.svg)<br>`harbour` | ![The lanterns scene: Paper lanterns rising slowly over a dark lake, with their reflections on the water](docs/images/scenes/lanterns.svg)<br>`lanterns` |
| ![The balloons scene: Hot air balloons lifting slowly from behind rolling hills under drifting clouds](docs/images/scenes/balloons.svg)<br>`balloons` | ![The countryside scene: A steam train crossing open country, with hills, fields, trees and a fence sliding past](docs/images/scenes/countryside.svg)<br>`countryside` | ![The winter-cabin scene: A log cabin in the snow at night, with a lit window, chimney smoke, pines and falling snow](docs/images/scenes/winter-cabin.svg)<br>`winter-cabin` |
| ![The autumn scene: A tree in autumn colour dropping leaves that drift down and gather on the ground](docs/images/scenes/autumn.svg)<br>`autumn` | ![The blossom scene: A flowering branch letting petals go on a breeze, down to a stream that carries them off](docs/images/scenes/blossom.svg)<br>`blossom` | ![The city-night scene: A night skyline where windows light and darken one at a time, a moon, a river and a passing train](docs/images/scenes/city-night.svg)<br>`city-night` |
| ![The orbit scene: A ringed planet among distant stars with a small moon on a slow orbit around it](docs/images/scenes/orbit.svg)<br>`orbit` | ![The constellations scene: Stars that are slowly joined into named figures one group at a time, then fade](docs/images/scenes/constellations.svg)<br>`constellations` | ![The tea scene: A mug of tea with curling steam beside books and a plant, with weather at the window](docs/images/scenes/tea.svg)<br>`tea` |
| ![The cat-nap scene: A cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window](docs/images/scenes/cat-nap.svg)<br>`cat-nap` | ![The garden scene: Seeds that sprout, grow and bloom over a couple of minutes, with butterflies, before the bed starts again](docs/images/scenes/garden.svg)<br>`garden` | ![The tiles scene: A grid of small folded pages in the theme's colours, one tile changing at a time](docs/images/scenes/tiles.svg)<br>`tiles` |
| ![The ribbons scene: Broad ribbons of contour lines flowing slowly across a faint grid of dots](docs/images/scenes/ribbons.svg)<br>`ribbons` |   |   |

### Fail scenes

| | | |
| --- | --- | --- |
| ![The storm scene: Heavy slanting rain under two rows of cloud, with distant lightning unless motion is reduced](docs/images/scenes/storm.svg)<br>`storm` | ![The flood scene: A street where the water rises slowly past the doors, with debris drifting on the surface](docs/images/scenes/flood.svg)<br>`flood` | ![The listing-ship scene: A cargo ship on a grey sea that slowly lists to one side as its waterline creeps up](docs/images/scenes/listing-ship.svg)<br>`listing-ship` |
| ![The corrosion scene: Riveted steel plates where rust creeps out from the rivets cell by cell and slowly drips](docs/images/scenes/corrosion.svg)<br>`corrosion` | ![The tangle scene: Cables hanging from a patch panel that slowly wander, cross and knot into a snarl](docs/images/scenes/tangle.svg)<br>`tangle` | ![The wilt scene: Potted plants on a sunny sill whose stems droop, leaves fall and soil cracks](docs/images/scenes/wilt.svg)<br>`wilt` |
| ![The fog scene: A road where a fog bank rolls in and the signs along the verge fade from view](docs/images/scenes/fog.svg)<br>`fog` | ![The broken-bridge scene: A bridge with its middle span gone, girders hanging, a barrier and a waiting car over a river](docs/images/scenes/broken-bridge.svg)<br>`broken-bridge` | ![The leak scene: A cracked pipe dripping into a bucket that slowly fills and overflows into a spreading puddle](docs/images/scenes/leak.svg)<br>`leak` |
| ![The blackout scene: A night skyline whose windows go dark one by one beside a snapped power line, leaving candles](docs/images/scenes/blackout.svg)<br>`blackout` | ![The traffic-jam scene: Cars queued bumper to bumper before a red signal and a row of cones, exhaust drifting up](docs/images/scenes/traffic-jam.svg)<br>`traffic-jam` | ![The erosion scene: A cliff edge crumbling into the sea, stones tumbling down and a fence left hanging](docs/images/scenes/erosion.svg)<br>`erosion` |
| ![The drought scene: Cracked earth spreading under a heavy sun, a dry riverbed with its last puddle and a bare tree](docs/images/scenes/drought.svg)<br>`drought` |   |   |

### In a light theme

![The lanterns scene in Zacess Light: paper lanterns rising over a lake, drawn in dark colours on a white background](docs/images/scenes/lanterns-light.svg)

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
signal. In the watch mode every tone is paired with the same state on screen, sound can be
turned off, made quieter or kept to waking hours and `--notify` sends each change as a desktop
notification a screen reader announces. Lua scenes are held to the same contrast and flash limits
while they run. Every colour is checked against its background in the tests. The full statement,
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
