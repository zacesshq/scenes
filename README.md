# Zacess Scenes

Calm animated scenes for the terminal, drawn in the eight [Zacess](https://zacess.com) high contrast
themes. It is also a build monitor for projects in any language and can follow a shell, showing
how each command ended as a scene.

| While the build passes | While it fails |
| --- | --- |
| ![The watch mode while a build passes: a calm sea under a sun with a large ALL GOOD banner, a check mark, a caption and a status bar reading passing](docs/images/scenes/watch-pass.svg) | ![The watch mode while a build fails: a storm with a large BUILD FAILED banner, a cross, two errors with their locations and a status bar reading failing](docs/images/scenes/watch-fail.svg) |

`zacess-scenes` is one small program. It draws a scene with text characters in the colours of the
Zacess theme the terminal is using, keeps every colour at 4.5 to 1 contrast or better (7 to 1 on
the High Contrast and Deep Contrast themes), never flashes and slows down when the system or the
person asks for reduced motion. A scene fits any terminal from 20 columns by 8 rows up to a full
screen.

> [!NOTE]
> The Homebrew tap and the winget package are being set up; until then install with Cargo or from
> a release binary.

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

## Documentation

| Page | Covers |
| --- | --- |
| [Using Zacess Scenes](docs/usage.md) | The commands, the keys, every flag and headless rendering |
| [Watch mode](docs/watch-mode.md) | The build monitor for any language: the command for each project, bacon for Rust, the status bar, choosing scenes, keys and notifications |
| [Follow mode](docs/follow-mode.md) | Every command in a shell as a result: the hook for zsh, bash, fish and PowerShell, channels and typos |
| [Sound sets](docs/sounds.md) | The eight sound sets, the events, the background sound and the sound settings |
| [Configuration](docs/configuration.md) | The `scenes.toml` file with a full example |
| [Themes](docs/themes.md) | How the theme is picked, the eight themes and colour depth |
| [Lua scenes](docs/lua.md) | Writing a scene of your own in Lua |
| [Snapshots](docs/snapshots.md) | SVG stills and JSON frames of any scene |
| [The scenes](docs/scenes.md) | Every scene in detail, with its colours and its tests |

## Scenes

39 scenes: the original ten, 16 calm pass scenes and 13 slow fail scenes that never flash. Each
runs on its own and in the watch mode the pass scenes show while a build passes and the fail
scenes while it fails. Every still is Zacess Deep Contrast Dark at 80 by 24, six seconds in.
[docs/scenes.md](docs/scenes.md) describes each one, with what changes under reduced motion.

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

### With Cargo

Zacess Scenes is not on crates.io yet, so Cargo installs it from a release tag here:

```sh
cargo install --locked --git https://github.com/zacesshq/scenes --tag v0.4.0 zacess-scenes
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
shasum -a 256 -c zacess-scenes-v0.4.0-aarch64-apple-darwin.tar.gz.sha256
tar -xzf zacess-scenes-v0.4.0-aarch64-apple-darwin.tar.gz
```

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
