<h1>
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/scenes-lockup-dark.svg">
    <img src="assets/scenes-lockup-light.svg" alt="Zacess Scenes" width="400">
  </picture>
</h1>

Calm animated scenes for the terminal, drawn in the ten [Zacess](https://zacess.com) accessible
themes. It is also a build monitor for projects in any language and can follow a shell, showing
how each command ended as a scene.

[![CI](https://github.com/zacesshq/scenes/actions/workflows/ci.yml/badge.svg)](https://github.com/zacesshq/scenes/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/zacesshq/scenes?label=release)](https://github.com/zacesshq/scenes/releases/latest)
[![Licence: MIT](https://img.shields.io/badge/licence-MIT-green.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/zacess-scenes)](https://crates.io/crates/zacess-scenes)

| While the build passes | While it fails |
| --- | --- |
| ![The watch mode while a build passes: a calm sea under a sun with a large ALL GOOD banner, a check mark, a caption and a status bar reading passing](docs/images/scenes/watch-pass.svg) | ![The watch mode while a build fails: a storm with a large BUILD FAILED banner, a cross, two errors with their locations and a status bar reading failing](docs/images/scenes/watch-fail.svg) |

`zacess-scenes` is one small program. It paints a scene in solid pixels, two to a character cell,
in the colours of the Zacess theme the terminal is using. Text keeps the theme's text contrast: 7
to 1 on eight themes and 4.5 to 1 on the two Warm themes. Background imagery stays below the 3 to
1 non-text contrast floor and the pixel shapes that carry meaning keep at least 3 to 1 against
the background. Many scenes have a look of their own on
the light themes, such as a day or a sunrise in place of a night. It never flashes and slows down
when the system or the person asks for reduced motion. A scene fits any terminal from 20 columns
by 8 rows up to a full screen.

## Quick start

```sh
zacess-scenes                       # the sea, in the active theme
zacess-scenes koi-pond              # one scene by name
zacess-scenes list                  # every scene and sound set
zacess-scenes --shuffle --ambient   # a new scene every five minutes, with background sound
cd my-project && zacess-scenes watch --sounds piano --notify
zacess-scenes follow                # how each command in this tmux session ended
```

While a scene runs `n` shows the next scene, `b` the one before, `a` turns the background sound on
or off, `Space` pauses and `q` quits. Follow mode needs a one-line hook in the shell:
`eval "$(zacess-scenes follow init zsh)"` (see [follow mode](docs/follow-mode.md)).

## How it works

```mermaid
flowchart LR
    subgraph SETTINGS["Settings"]
        FLAGS["Flags and<br/>environment variables"]
        CONFIG["scenes.toml"]
        THEME["Theme state file<br/>shared with the Zacess themes"]
    end
    subgraph FOLLOWED["What a scene can follow"]
        BUILD["A project's own check<br/>bacon, npm, pytest, go, make"]
        SHELL["The shell hook<br/>zsh, bash, fish, PowerShell"]
    end
    SETTINGS --> APP["zacess-scenes"]
    BUILD -- "watch" --> APP
    SHELL -- "follow" --> APP
    APP --> SCENE["Scene<br/>built in or written in Lua"]
    SCENE --> RENDER["Renderer<br/>palette roles at the theme floor,<br/>flash meter, reduced motion"]
    RENDER --> TERM["Terminal frame<br/>with the status bar and captions"]
    RENDER --> SVG["SVG still or JSON frames<br/>from snapshot"]
    APP -- "pass, fail, recovered, typo" --> SOUND["Tones, background sound<br/>and desktop notifications"]
```

A scene never picks a colour of its own. It asks for a palette role (`text`, `muted`, `accent`,
`blue` and so on) and the renderer looks that role up in the active theme, so every colour keeps
the contrast floor whichever theme is on and whoever wrote the scene. A terminal colour below the
floor is fitted to it while keeping its own hue wherever it can.

## Documentation

| Page | Covers |
| --- | --- |
| [Using Zacess Scenes](docs/usage.md) | The commands, the keys, every flag and headless rendering |
| [Watch mode](docs/watch-mode.md) | The build monitor for any language: the command for each project, bacon for Rust, the status bar, choosing scenes, keys and notifications |
| [Follow mode](docs/follow-mode.md) | Every command in a shell as a result: the hook for zsh, bash, fish and PowerShell, channels and typos |
| [Sound sets](docs/sounds.md) | The eight sound sets, the events, the background sound and the sound settings |
| [Configuration](docs/configuration.md) | The `scenes.toml` file with a full example |
| [Themes](docs/themes.md) | How the theme is picked, the ten themes and colour depth |
| [Lua scenes](docs/lua.md) | Writing a scene of your own in Lua |
| [Snapshots](docs/snapshots.md) | SVG stills and JSON frames of any scene |
| [The scenes](docs/scenes.md) | Every scene in detail, with its colours and its tests |

## Scenes

65 scenes: the original ten and four more general scenes, 27 calm pass scenes and 24 slow fail
scenes that never flash. Each runs on its own and in the watch mode the pass scenes show while a
build passes and the fail scenes while it fails. Every still is Zacess Deep Contrast Dark at 80 by
24, six seconds in. [docs/scenes.md](docs/scenes.md) describes each one, with what changes under
reduced motion.

### The original scenes

| | | |
| --- | --- | --- |
| ![The sea scene: Fish of three sizes, bubbles, seaweed, a boat on the surface and an occasional whale, under a daylit sky in light themes](docs/images/scenes/sea.svg)<br>`sea` | ![The rain scene: Rain on a lamplit evening street with puddles, a walker under an umbrella and distant lightning, an overcast afternoon in light themes](docs/images/scenes/rain.svg)<br>`rain` | ![The snow scene: Flakes at three depths drifting over far hills and pines, settling into a bank along the bottom, by day in light themes](docs/images/scenes/snow.svg)<br>`snow` |
| ![The starfield scene: Twinkling stars over a faint band of the galaxy and low hills, a crescent moon and a slow shooting star, at dawn in light themes](docs/images/scenes/starfield.svg)<br>`starfield` | ![The code-rain scene: Falling columns of characters in the theme's syntax colours, each with a bright head, over a softly shaded backdrop](docs/images/scenes/code-rain.svg)<br>`code-rain` | ![The fireflies scene: Fireflies glowing over a meadow at dusk under a moon and a great tree, by day in light themes with butterflies and flowers](docs/images/scenes/fireflies.svg)<br>`fireflies` |
| ![The aurora scene: Slow curtains of colour across a starry sky over snow-capped mountains, at the blue hour in light themes](docs/images/scenes/aurora.svg)<br>`aurora` | ![The campfire scene: A campfire in a pine clearing, flames over logs in a ring of stones with rising embers, on a late afternoon in light themes](docs/images/scenes/campfire.svg)<br>`campfire` | ![The waves scene: A calm sea to the horizon under a sun, with gulls, the sun a pale disc in a warm halo in light themes](docs/images/scenes/waves.svg)<br>`waves` |
| ![The zacess scene: The Zacess mark floating slowly round the middle of the screen over a soft wash with two flowing ribbons of colour](docs/images/scenes/zacess.svg)<br>`zacess` |  |  |

### More general scenes

Drawn in solid pixels, like `sea`.

| | | |
| --- | --- | --- |
| ![The forest-stream scene: A stream winding through a forest, light slanting through the trees and leaves on the water](docs/images/scenes/forest-stream.svg)<br>`forest-stream` | ![The desert-night scene: Desert dunes under a starry sky, a moon rising slowly and camels walking the far ridge, at sunrise in light themes](docs/images/scenes/desert-night.svg)<br>`desert-night` | ![The rooftops scene: Rooftops at dusk with chimney smoke, pigeons on the ridges and windows lighting up, in late afternoon in light themes](docs/images/scenes/rooftops.svg)<br>`rooftops` |
| ![The lily-pads scene: A pond of lily pads from above, frogs hopping between them and dragonflies darting about](docs/images/scenes/lily-pads.svg)<br>`lily-pads` |  |  |

### Pass scenes

| | | |
| --- | --- | --- |
| ![The koi-pond scene: A garden pond from above, with shaded koi gliding past lily pads and slow ripples](docs/images/scenes/koi-pond.svg)<br>`koi-pond` | ![The harbour scene: A harbour town at night, with shaded boats, quay lamps, a lighthouse and their reflections, on a sunny afternoon in light themes](docs/images/scenes/harbour.svg)<br>`harbour` | ![The lanterns scene: Glowing paper lanterns rising slowly over a dark lake, with their reflections on the water, at dusk in light themes](docs/images/scenes/lanterns.svg)<br>`lanterns` |
| ![The balloons scene: Striped hot air balloons lifting slowly from behind rolling hills under soft drifting clouds, at dawn or by day in light themes](docs/images/scenes/balloons.svg)<br>`balloons` | ![The countryside scene: A steam train crossing open country at evening, with hills, fields, trees and a fence sliding past, by day in light themes](docs/images/scenes/countryside.svg)<br>`countryside` | ![The winter-cabin scene: A log cabin in the snow at night, with a lit window, chimney smoke, snowy pines and falling snow, in the afternoon in light themes](docs/images/scenes/winter-cabin.svg)<br>`winter-cabin` |
| ![The autumn scene: A shaded tree in autumn colour dropping leaves that drift down and gather on the ground, in the afternoon in light themes](docs/images/scenes/autumn.svg)<br>`autumn` | ![The blossom scene: A flowering branch letting petals go on a breeze, down to a stream that carries them off, on a clear day in light themes](docs/images/scenes/blossom.svg)<br>`blossom` | ![The city-night scene: A painted night skyline where windows light and darken one at a time, a moon, a river and a passing train, at dusk in light themes](docs/images/scenes/city-night.svg)<br>`city-night` |
| ![The orbit scene: A shaded, ringed planet among distant stars with a small moon on a slow orbit around it, on a pale sky in light themes](docs/images/scenes/orbit.svg)<br>`orbit` | ![The constellations scene: Stars in a painted night sky over wooded hills, joined into named figures one group at a time, at the blue hour in light themes](docs/images/scenes/constellations.svg)<br>`constellations` | ![The tea scene: A painted mug of tea with curling steam beside books and a plant, with weather at the window](docs/images/scenes/tea.svg)<br>`tea` |
| ![The cat-nap scene: A painted cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window, by day in light themes](docs/images/scenes/cat-nap.svg)<br>`cat-nap` | ![The garden scene: A flower bed by a picket fence that sprouts, grows and blooms over a couple of minutes, with butterflies, before it starts again](docs/images/scenes/garden.svg)<br>`garden` | ![The tiles scene: A grid of painted folded pages in the theme's colours, one tile changing at a time](docs/images/scenes/tiles.svg)<br>`tiles` |
| ![The ribbons scene: Broad ribbons of shaded silk twisting and flowing slowly across the screen](docs/images/scenes/ribbons.svg)<br>`ribbons` | ![The sunflower-field scene: Rows of sunflowers swaying under a warm sky, with bees drifting between the heads](docs/images/scenes/sunflower-field.svg)<br>`sunflower-field` | ![The mountain-lake scene: Mountains and pines mirrored in a still lake at sunrise, with mist on the water, a slow ripple and a heron fishing](docs/images/scenes/mountain-lake.svg)<br>`mountain-lake` |
| ![The tide-pools scene: Rock pools at low tide with crabs on the sand, anemones and starfish and a gentle shimmer](docs/images/scenes/tide-pools.svg)<br>`tide-pools` | ![The paper-boats scene: Folded paper boats drifting down a stream past swaying reeds, the current carrying them on](docs/images/scenes/paper-boats.svg)<br>`paper-boats` | ![The windmills scene: A row of windmills turning slowly over a patchwork of fields as clouds pass overhead](docs/images/scenes/windmills.svg)<br>`windmills` |
| ![The picnic scene: A picnic blanket on a grassy hill, with kites flying overhead and clouds drifting by](docs/images/scenes/picnic.svg)<br>`picnic` | ![The drone-landing scene: A quadcopter mapping rolling ground at dusk with a faint beam, then settling on a marked landing pad, by day in light themes](docs/images/scenes/drone-landing.svg)<br>`drone-landing` | ![The kettle-boil scene: A kettle on a kitchen counter that boils, clicks off and pours into a mug, in the evening or by day in light themes](docs/images/scenes/kettle-boil.svg)<br>`kettle-boil` |
| ![The rocket-launch scene: A rocket counting down on a coastal pad at dusk, lifting off in a billow of smoke on a long trail, by day in light themes](docs/images/scenes/rocket-launch.svg)<br>`rocket-launch` | ![The pit-stop scene: A race car sliding into its pit box at night, the crew changing its wheels as the lollipop lifts, by day in light themes](docs/images/scenes/pit-stop.svg)<br>`pit-stop` | ![The pcb-bring-up scene: A circuit board on a lamp-lit bench lighting its LEDs in turn until the status LED glows green, by day in light themes](docs/images/scenes/pcb-bring-up.svg)<br>`pcb-bring-up` |

### Fail scenes

| | | |
| --- | --- | --- |
| ![The storm scene: Slanting rain under rolling banks of storm cloud, a tree in full leaf bending in the wind and lightning, by day in light themes](docs/images/scenes/storm.svg)<br>`storm` | ![The flood scene: A street where muddy water rises slowly past the doors and a parked car under a grey drizzle, with debris bobbing on the surface](docs/images/scenes/flood.svg)<br>`flood` | ![The listing-ship scene: A container ship on a grey sea that slowly lists bow down in the rain as its waterline creeps up](docs/images/scenes/listing-ship.svg)<br>`listing-ship` |
| ![The corrosion scene: Riveted steel plates where rust creeps out from the rivets, flakes and runs down in streaks](docs/images/scenes/corrosion.svg)<br>`corrosion` | ![The tangle scene: Round cables hanging from a patch panel that slowly wander, cross and knot into a snarl](docs/images/scenes/tangle.svg)<br>`tangle` | ![The wilt scene: Potted plants on a sunny sill whose stems droop and dry, leaves fall and soil cracks](docs/images/scenes/wilt.svg)<br>`wilt` |
| ![The fog scene: A road at dusk where a bank of fog rolls in and swallows the lamps and signs along the verge](docs/images/scenes/fog.svg)<br>`fog` | ![The broken-bridge scene: A concrete bridge with its middle span gone, girders hanging, a striped barrier and a waiting car over a river](docs/images/scenes/broken-bridge.svg)<br>`broken-bridge` | ![The leak scene: A cracked pipe in a basement dripping into a pail that fills, brims over and spreads into a puddle](docs/images/scenes/leak.svg)<br>`leak` |
| ![The blackout scene: A night skyline whose windows go dark one by one beside a snapped power line, leaving candles, at dusk in light themes](docs/images/scenes/blackout.svg)<br>`blackout` | ![The traffic-jam scene: Cars and vans queued bumper to bumper with brake lights on before a red signal, exhaust drifting up, by day in light themes](docs/images/scenes/traffic-jam.svg)<br>`traffic-jam` | ![The erosion scene: A layered cliff crumbling into the sea, stones tumbling down and a fence left hanging over the edge](docs/images/scenes/erosion.svg)<br>`erosion` |
| ![The drought scene: A parched plain cracked into curling plates under a heavy sun, a dry riverbed with its last puddle and a dead tree](docs/images/scenes/drought.svg)<br>`drought` | ![The volcano scene: A volcano at dusk with a glowing crater and a billowing plume of ash, three lava flows creeping down its gullies to pool at the foot](docs/images/scenes/volcano.svg)<br>`volcano` | ![The rockslide scene: Boulders breaking from a fresh scar and tumbling down a mountainside to heap across the road, by day in light themes](docs/images/scenes/rockslide.svg)<br>`rockslide` |
| ![The dust-storm scene: A towering wall of dust rolling over the desert, swallowing the far poles, the mesas, the dunes and the sun, by day in light themes](docs/images/scenes/dust-storm.svg)<br>`dust-storm` | ![The thin-ice scene: Cracks spreading in a star across a frozen lake from one weak point and a hole of dark water opening, under a moon or a low winter sun](docs/images/scenes/thin-ice.svg)<br>`thin-ice` | ![The short-circuit scene: A circuit board where one trace has burnt through, its copper blackening, a scorch spreading and slow sparks rising](docs/images/scenes/short-circuit.svg)<br>`short-circuit` |
| ![The overgrown scene: Weeds and vines slowly swallowing a garden path and its gate until the way through is lost](docs/images/scenes/overgrown.svg)<br>`overgrown` | ![The landing-abort scene: The same quadcopter finding no terrain match, hovering over the pad at dusk, then climbing calmly away, by day in light themes](docs/images/scenes/landing-abort.svg)<br>`landing-abort` | ![The kettle-cold scene: The same kettle clicking off before it boils, no steam, with a note by the mug to try again, in the evening or by day in light themes](docs/images/scenes/kettle-cold.svg)<br>`kettle-cold` |
| ![The launch-scrub scene: The same coastal pad with the rocket held, venting steam as the gantry arm swings back and the board reads HOLD, by day in light themes](docs/images/scenes/launch-scrub.svg)<br>`launch-scrub` | ![The pit-stop-stuck scene: The same pit stop with one wheel that will not come off, the crew gathered round and the board climbing, by day in light themes](docs/images/scenes/pit-stop-stuck.svg)<br>`pit-stop-stuck` | ![The dead-led scene: The same bench with one LED in the chain left dark, a probe moving to it and a flat trace on the scope, by day in light themes](docs/images/scenes/dead-led.svg)<br>`dead-led` |

### In a light theme

![The lanterns scene in Zacess Light: paper lanterns in bright painted colours rising over a lake under a violet blue dusk sky that pales to peach](docs/images/scenes/lanterns-light.svg)

## Sound sets

The watch mode plays a short tone for each change in a build, in one of eight sets. Every set plays
the same phrase for each event, so the meaning carries over; only the timbre differs.
[docs/sounds.md](docs/sounds.md) has the events and the settings.

| Set | Sounds like |
| --- | --- |
| `bells` (the default) | Soft bells with a little shimmer, the original tones |
| `marimba` | Warm wooden bars, short and round |
| `kalimba` | Plucked metal tines with a gentle ring |
| `pad` | A soft synth pad that swells in and fades out |
| `chip` | Gentle 8-bit square waves, rounded so they never buzz |
| `glass` | Slow glassy tones like a wet finger on a rim |
| `piano` | A soft grand piano, felt hammers and a warm low ring |
| `rain` | Water droplets over a quiet bed of rain |

## Install

| System | Shortest route |
| --- | --- |
| macOS and Linux | [Homebrew](#homebrew-macos-and-linux) |
| Windows | [winget](#winget-windows) |
| Anywhere with a Rust toolchain | [Cargo](#cargo-cratesio) |
| Anywhere else and any machine without a package manager | [A release archive](#a-release-archive) |

### Homebrew (macOS and Linux)

```sh
brew install zacesshq/tap/zacess-scenes
```

The formula installs the release binary for the machine (Apple silicon, Intel, Linux on x86-64 or
ARM64) after Homebrew has checked it against the checksum recorded in the formula. `brew upgrade
zacess-scenes` moves to a newer release.

### winget (Windows)

```powershell
winget install Zacess.Scenes
```

Each release is submitted to the [winget community repository](https://github.com/microsoft/winget-pkgs)
once its files are attached, so the newest version can take a day or two to reach winget. The
release archive below is current straight away.

### Cargo (crates.io)

```sh
cargo install --locked zacess-scenes
```

Rust 1.85 or later and a C compiler, since Lua 5.4 is built into the program. The dependencies are
crossterm, mlua with its own copy of Lua and signal-hook on Unix. The crate on
[crates.io](https://crates.io/crates/zacess-scenes) carries the code, the docs and the licence; the
gallery stills stay here.

### A release archive

Each [release](https://github.com/zacesshq/scenes/releases/latest) carries a built binary for:

| File | System |
| --- | --- |
| `zacess-scenes-<tag>-aarch64-apple-darwin.tar.gz` | macOS on Apple silicon |
| `zacess-scenes-<tag>-x86_64-apple-darwin.tar.gz` | macOS on Intel |
| `zacess-scenes-<tag>-x86_64-unknown-linux-musl.tar.gz` | Linux on x86-64, no shared libraries needed |
| `zacess-scenes-<tag>-aarch64-unknown-linux-musl.tar.gz` | Linux on ARM64, no shared libraries needed |
| `zacess-scenes-<tag>-x86_64-pc-windows-msvc.zip` | Windows on x86-64 |

Each archive has a `.sha256` file beside it. Download both into the same folder, check the sum,
unpack the archive and put `zacess-scenes` (or `zacess-scenes.exe`) somewhere on your `PATH`.

On macOS:

```sh
shasum -a 256 -c zacess-scenes-v1.1.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf zacess-scenes-v1.1.0-aarch64-apple-darwin.tar.gz
install -m 755 zacess-scenes-v1.1.0-aarch64-apple-darwin/zacess-scenes ~/.local/bin/
```

On Linux:

```sh
sha256sum -c zacess-scenes-v1.1.0-x86_64-unknown-linux-musl.tar.gz.sha256
tar -xzf zacess-scenes-v1.1.0-x86_64-unknown-linux-musl.tar.gz
install -m 755 zacess-scenes-v1.1.0-x86_64-unknown-linux-musl/zacess-scenes ~/.local/bin/
```

On Windows, in PowerShell, the two hashes printed must match:

```powershell
(Get-FileHash .\zacess-scenes-v1.1.0-x86_64-pc-windows-msvc.zip).Hash
Get-Content .\zacess-scenes-v1.1.0-x86_64-pc-windows-msvc.zip.sha256
Expand-Archive .\zacess-scenes-v1.1.0-x86_64-pc-windows-msvc.zip -DestinationPath .
```

### From a script

A setup script can keep a machine on the newest release without a package manager. The release
names follow one pattern, so the script asks GitHub for the latest tag, downloads the archive for
its own platform with the `.sha256` file and checks the sum before installing:

```sh
tag="$(curl -fsSL https://api.github.com/repos/zacesshq/scenes/releases/latest | sed -n 's/.*"tag_name": *"\(v[0-9.]*\)".*/\1/p')"
target=aarch64-apple-darwin   # or x86_64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl
name="zacess-scenes-$tag-$target"
curl -fsSLO "https://github.com/zacesshq/scenes/releases/download/$tag/$name.tar.gz"
curl -fsSLO "https://github.com/zacesshq/scenes/releases/download/$tag/$name.tar.gz.sha256"
shasum -a 256 -c "$name.tar.gz.sha256"   # sha256sum -c on Linux
tar -xzf "$name.tar.gz"
install -m 755 "$name/zacess-scenes" ~/.local/bin/
```

`zacess-scenes --version` prints the installed version, so a script can skip the download when it
already matches the tag.

## Accessibility

Nothing flashes and reduced motion is honoured from `--reduced-motion`, `ZACESS_REDUCED_MOTION=1`
and the system setting on macOS, GNOME and Windows. Every tone has the same state on screen, sound
can be turned off or kept to waking hours and `--notify` sends each change of a build as a desktop
notification a screen reader announces. The full statement, with the measured limits, is in
[ACCESSIBILITY.md](ACCESSIBILITY.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). A new scene implements one trait and is checked by the same
tests as the rest.

## Licence

[MIT](LICENSE). The licence does not cover the Zacess name or mark; see [NOTICE.md](NOTICE.md).
