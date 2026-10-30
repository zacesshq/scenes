# Configuration

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
interval = 15           # minutes; 0 changes only on a new result (also a scene run on its own)
motion = "standard"     # calm, standard or lively
reduced_motion = false
captions = true
banner = true           # off draws ALL GOOD or BUILD FAILED on one line instead

[sound]
enabled = true
set = "kalimba"         # bells, marimba, kalimba, pad, chip, glass, piano or rain
volume = 40
quiet_hours = "22:00-07:00"

[sound.events]
pass = true
fail = true
tick = false
recovered = true
still_failing = false
warnings = true
typo = true             # a command a followed shell could not find

[notifications]
enabled = true

[ambient]
enabled = true          # a quiet background sound that suits the scene
volume = 20

[follow]
min_seconds = 0         # leave out followed commands quicker than this
```

With `rotation` set to `cycle` or `shuffle` and an `interval` above 0, a scene run on its own with
no name moves through the `pass` list by itself, which makes a slideshow. Without a `pass` list it
moves through every pass and general scene.

The file is read by a small parser of our own for the part of TOML it needs: tables, keys,
strings, numbers, booleans and lists, with comments. That keeps the program to the dependencies it
already had plus Lua.
