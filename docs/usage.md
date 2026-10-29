# Using Zacess Scenes

Running a scene on its own: the commands, the keys, every flag and headless rendering. The
[watch mode](watch-mode.md) has keys and settings of its own.

## Run a scene

```sh
zacess-scenes                       # the sea, in the active theme
zacess-scenes campfire              # one scene by name
zacess-scenes lanterns --motion calm
zacess-scenes snow --theme cream    # in a named theme
zacess-scenes aurora --duration 10  # stops on its own after ten minutes
zacess-scenes list                  # every scene and sound set
zacess-scenes --list-themes
```

## Keys

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

## Commands

| Command | Does |
| --- | --- |
| `zacess-scenes watch [job]` | Shows the state of a Rust build as a scene. See [watch mode](watch-mode.md) |
| `zacess-scenes list` | Prints every scene (marked pass, fail or general) and the sound sets |
| `zacess-scenes config init` | Writes a fully commented config file. Add `--force` to replace one |
| `zacess-scenes config path` | Prints where the config file is |
| `zacess-scenes sounds play <set> [event]` | Plays a sound set's tone for an event, `pass` by default |
| `zacess-scenes scene new <name>` | Writes a commented starter Lua scene into the scenes folder |
| `zacess-scenes snapshot <scene>` | Draws a scene into an SVG still or JSON frames. See [snapshots](snapshots.md) |

## Flags

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
