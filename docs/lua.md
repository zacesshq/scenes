# Writing a scene in Lua

Zacess Scenes can run scenes of your own, written in [Lua 5.4](https://www.lua.org/manual/5.4/).
Lua is compiled into the program, so there is nothing to install. A user scene is listed by
`zacess-scenes list` and runs on its own like a built-in scene. It can also be picked for the watch
mode in `scenes.toml`, with `--scene` or with `--fail-scene`.

A user scene never chooses a colour. It names a role from the palette, such as `accent` or
`syntax.keyword`. The program draws that role in the active Zacess theme, which keeps a user
scene's text inside the same contrast floor as the built-in scenes: the text contrast the palette
sets for each theme, 7 to 1 on the first eight themes and 4.5 to 1 on Warm Dark and Warm Light. The same
flash limit applies too, measured while the scene runs.

## Where scenes live

Each scene is one `.lua` file in the `scenes` folder next to `scenes.toml`:

| System | Folder |
| --- | --- |
| macOS and Linux | `$XDG_CONFIG_HOME/zacess/scenes` when `XDG_CONFIG_HOME` is set, otherwise `~/.config/zacess/scenes` |
| Windows | `%APPDATA%\zacess\scenes` |

Every `.lua` file there is loaded when the program starts, in name order. A file that fails to load
is reported and left out; the rest still load.

## Start a scene

```sh
zacess-scenes scene new pond
```

This writes `pond.lua` into the scenes folder with every part of the API commented. It refuses to
overwrite a file that is already there. Run the new scene with:

```sh
zacess-scenes pond
zacess-scenes watch --scene pond
```

A finished example, a paper lantern swaying over still water, is in
[examples/scenes/lantern.lua](../examples/scenes/lantern.lua). Copy it into the scenes folder to run
it.

## The script

A script returns a table. Only `frame` is required.

```lua
return {
  name = "pond",
  kind = "pass",
  summary = "Slow ripples on a pond at dusk",
  captions = { "All good.", "Still water and a clean build." },
  init = function(ctx) end,
  resize = function(ctx) end,
  frame = function(ctx) end,
  on_pass = function(ctx) return "Passed." end,
  on_fail = function(ctx) return "Something to fix." end,
  on_recovered = function(ctx) return "Fixed." end,
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `name` | string | The name used on the command line and in `scenes.toml`. Letters, digits and hyphens; `_` and spaces become hyphens and capitals become lower case. It may not be the name of a built-in scene. Defaults to the file name without `.lua` |
| `kind` | string | `pass` for a passing build, `fail` for a failing one or `general` for either. Defaults to `general` |
| `summary` | string | One line shown by `zacess-scenes list` |
| `captions` | list of strings | Lines shown under the watch mode's banner, one per run in turn. Each is cut to 60 characters. Without any, the watch mode's own captions are used |
| `init` | function | Called once, before the first frame |
| `resize` | function | Called when the terminal changes size. `ctx.width` and `ctx.height` are already the new size |
| `frame` | function | Called once a frame, about 20 times a second. Move things on by `ctx.dt` seconds and draw them |
| `on_pass` | function | Called when the watch mode shows a passing build. May return a caption |
| `on_fail` | function | Called when the watch mode shows a failing build. May return a caption |
| `on_recovered` | function | Called when a failing build passes again. May return a caption |

A hook that returns a string replaces the caption for that moment. Returning nothing keeps the
caption from `captions`.

State lives in ordinary Lua variables, such as locals declared above the `return`. They keep their
values from one frame to the next.

## The context

Every function is called with one argument, `ctx`:

| Field | Type | Meaning |
| --- | --- | --- |
| `ctx.width` | integer | Columns on screen |
| `ctx.height` | integer | Rows on screen |
| `ctx.time` | number | Seconds since the scene started |
| `ctx.dt` | number | Seconds since the last frame, 0 in `init` and `resize` |
| `ctx.motion` | string | The motion level: `calm`, `standard` or `lively` |
| `ctx.reduced` | boolean | Whether reduced motion is on |
| `ctx.speed` | number | The factor to multiply movement by: 1 on standard, less on calm or under reduced motion and more on lively |
| `ctx.pace` | number | The factor for anything that changes in place, such as a pulse or a glow. It is never above 1, so lively cannot make a cell change faster |
| `ctx.busy` | number | The factor to multiply counts by: fewer things on calm and more on lively |
| `ctx.random()` | function | A number from 0 up to but not including 1 |
| `ctx.random(n)` | function | A whole number from 1 to `n` |
| `ctx.random(a, b)` | function | A whole number from `a` to `b` |
| `ctx.put(x, y, text, role)` | function | Draws text, see below. `put` on its own does the same |
| `ctx.pixel_width` | integer | Pixels across the canvas, the same as `ctx.width` |
| `ctx.pixel_height` | integer | Pixels down the canvas, twice `ctx.height` |
| `ctx.pixel`, `ctx.rect`, `ctx.disc`, `ctx.line`, `ctx.blend` | function | Paint solid pixels, see [Pixels](#pixels) |
| `ctx.clock` | table or nil | The local date and time, see [The clock](#the-clock) |

`ctx.random` and `math.random` share one generator seeded from the program's, so a run with
`--seed` repeats exactly. `math.randomseed` does nothing.

## Drawing

```lua
put(x, y, text, role)
```

- `x` and `y` are the column and row of the first character, counted from 0 at the top left.
  Fractions are rounded down. Anything outside the screen is left out, so a shape can sit half off
  an edge.
- `text` is drawn from left to right, spaces included. A number is drawn as its digits.
- `role` is a palette role name. It defaults to `text`.
- A character that some terminal fonts lack is drawn as `?`. The safe set is printable ASCII, the
  middle dot `·`, the block and shade characters from `▀` to `▟` and the light box drawing lines
  `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`.
- The screen starts empty each frame. Later calls draw over earlier ones.
- One frame may make at most 20,000 `put` calls of up to 1,000 characters each.

### Roles

| Role | Colour in the theme |
| --- | --- |
| `text` | The text colour (`foreground` also works) |
| `muted` | The muted text colour |
| `accent`, `link`, `info` | The interface colours of the same names |
| `error`, `warning` | The error and warning colours, with the theme's red or yellow where those are below the floor |
| `bright` | The strongest text colour: white on a dark theme and near black on a light one |
| `grey` | A grey that still reads on the background (`gray` also works) |
| `red`, `green`, `yellow`, `blue`, `magenta`, `cyan` | The terminal colours |
| `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan` | The bright terminal colours |
| `syntax.keyword`, `syntax.control`, `syntax.function`, `syntax.type`, `syntax.string`, `syntax.number`, `syntax.comment`, `syntax.property`, `syntax.regex` | The syntax colours |

Names ignore case and accept `-` for `_`. A name that is not a role, such as a hex colour, is an
error. A terminal colour below the theme's floor is already fitted to it, as it is for the built-in
scenes: by its bright or plain twin, otherwise by the same hue at a lightness that keeps the floor
(see [themes](themes.md)).

Colour is never the only signal in a Zacess scene. Give things different characters as well as
different roles, so the scene still reads with `NO_COLOR` set.

## Pixels

Text is not the only way to draw. The same canvas the built-in scenes paint on is open to a
script: each character cell holds two square pixels, one above the other, so the canvas is
`ctx.pixel_width` pixels across and `ctx.pixel_height` pixels down, counted from 0 at the top left.
Pixels suit solid imagery such as hills, clouds, water or a drone; text suits words and fine detail.

| Function | Paints |
| --- | --- |
| `ctx.pixel(x, y, role, strength)` | One pixel |
| `ctx.rect(x, y, width, height, role, strength)` | A filled rectangle |
| `ctx.disc(x, y, radius, role, strength)` | A filled disc centred on a point |
| `ctx.line(x1, y1, x2, y2, role, strength)` | A line one pixel wide |
| `ctx.blend(x, y, role, amount)` | Moves the pixel already there `amount` of the way towards the role's colour |

- The role is a palette role as for `put` and defaults to `text`. A pixel never takes a colour of
  its own, so imagery follows the theme like everything else.
- `strength` runs from 0 (the theme's background) to 1 (the role's full colour) and defaults to 1.
  Anything between is mixed in the OKLab space, so `0.25` gives a faint wash and `0.6` a soft
  shade. Keep background imagery faint and give anything that carries meaning full strength, so it
  stands out from the background as the built-in scenes do.
- Positions are rounded down and anything off the canvas is left out, so a shape can sit half off
  an edge. A rectangle or disc far bigger than the screen is cut to the screen.
- Pixels are painted first in the order they were called. Text is drawn over them. Text keeps
  the picture behind it only while it keeps its full contrast against it; where it would not, that
  cell falls back to the plain background. A space drawn over a picture leaves the picture as it
  is, so words can be laid out over imagery.
- The canvas starts empty each frame. One frame may make at most 50,000 pixel calls. Between them
  they may touch at most 64 screens' worth of pixels, counted after each shape is cut to the
  screen. Past that the rest of that frame's imagery is left out.
- Pixels count towards the flash limit like everything else, so a large area that lights up or
  goes dark at once is held back.

```lua
-- a soft cloud of overlapping discs and a sun, then words over the sky
ctx.disc(20, 8, 5, "muted", 0.35)
ctx.disc(27, 7, 6, "muted", 0.35)
ctx.disc(33, 9, 4, "muted", 0.35)
ctx.disc(ctx.pixel_width - 12, 10, 4, "yellow")
put(2, 1, "Good morning", "bright")
```

A scene that should still run on an older version, from before pixels, can check for them first
with `if ctx.rect then ... end`.

## The clock

`ctx.clock` is the local date and time in the system's time zone, refreshed once a second. It is a
fresh table each second, so changing a field changes nothing outside the script.

| Field | Example | Meaning |
| --- | --- | --- |
| `hour`, `minute`, `second` | `14`, `5`, `9` | The time on a 24 hour clock |
| `time` | `"14:05"` | The time as text |
| `year`, `month`, `day` | `2026`, `11`, `7` | The date |
| `weekday` | `6` | 1 for Monday to 7 for Sunday |
| `weekday_name`, `month_name` | `"Saturday"`, `"November"` | The names in English |
| `date` | `"Saturday 7 November 2026"` | The date as text |

It is `nil` when the system cannot say which time zone it is in, so a scene never shows a time
that might be hours out. It is missing too on versions from before the clock. A scene that uses it
checks first:

```lua
local c = ctx.clock
if c then
  local part = c.hour < 5 and "night" or c.hour < 12 and "morning" or c.hour < 18 and "afternoon" or "evening"
  put(2, 1, "Good " .. part .. ", it is " .. c.time, "text")
end
```

In a snapshot the clock does not read the system. It starts at Monday 1 June 2026 at 09:00 (or the
time `--clock` names) and moves on with `--time`, so a still is the same on every machine.
`--state pass` or `--state fail` runs the scene's `on_pass` or `on_fail` hook as it starts, so a
still can show what a scene does when a result arrives.

## The sandbox

A script can only compute and draw:

- The libraries are `string`, `table`, `math` and `utf8`, plus the safe parts of the base library.
- There is no `io`, `os`, `package`, `require`, `debug`, `coroutine`, `load`, `loadfile`,
  `dofile`, `collectgarbage`, `print` or `string.dump`. A script cannot read or write files, run
  programs, reach the network or load other code.
- Only text chunks are loaded, never precompiled bytecode.
- A Lua state may use at most 32 MB of memory.
- Every call has an instruction budget: 20 million instructions for loading the file and `init`
  and 2 million for each `frame`, `resize` and hook. A loop that never ends stops within a few
  milliseconds with an error instead of freezing the terminal.

## Flashing

Every frame a user scene draws passes through the same flash meter the built-in scenes are tested
against. The meter splits the screen into twelve regions and works out how bright each one looks
from the characters and the luminance of their colours. A large change in any region counts as one
step. At most three large steps are let through in any second, which is at most one and a half
flashes a second: half the limit in the guidelines. A frame that would go over the limit is held
back and the last frame shown stays on screen instead. A scene that has more than ten frames held
back within ten seconds is stopped with the message `<name> was stopped because it flashed too
often`.

Things that move, such as a drop falling or a boat drifting, keep each region about as bright as it
was and never count. To change something in place, such as a glow or a light in a window, change a
few cells at a time and keep the period in a named variable, as the example does with
`GLOW_PERIOD`. Multiply in-place rates by `ctx.pace` and movement by `ctx.speed`. Leave out
anything like lightning when `ctx.reduced` is true.

## Errors

A script that fails to load is reported with its file name and line, such as
`pond.lua:12: attempt to index a nil value`. It is left out of the scene list and
`zacess-scenes list` prints the reason.

A script that fails while it runs is stopped and never called again. That covers a runtime error,
an unknown role, the instruction budget, the memory limit and flashing. The last frame it drew
stays until the program swaps in a built-in scene. In the watch mode the status bar names the
scene and the problem, then a built-in scene of the same kind takes its place. The program never
crashes because of a script.

## A worked example

This scene draws a jar of fireflies on a shelf. The lights wander inside the jar and pulse slowly,
with the period kept in a variable and slowed by `ctx.pace`.

```lua
-- jar: fireflies in a glass jar on a shelf

local PULSE_PERIOD = 4 -- seconds per pulse, slow enough that no light blinks
local flies = {}

local function jar_box(ctx)
  local w = math.min(16, ctx.width - 2)
  local h = math.min(8, ctx.height - 2)
  local left = math.floor((ctx.width - w) / 2)
  local top = ctx.height - 1 - h
  return left, top, w, h
end

return {
  name = "jar",
  kind = "pass",
  summary = "Fireflies pulsing slowly in a glass jar on a shelf",
  captions = { "Small lights and a clean build.", "All good in the jar." },

  init = function(ctx)
    for i = 1, math.max(2, math.floor(5 * ctx.busy)) do
      flies[i] = { x = ctx.random(), y = ctx.random(), phase = ctx.random() * 6.28 }
    end
  end,

  frame = function(ctx)
    local left, top, w, h = jar_box(ctx)
    -- the shelf, then the lid and the sides of the jar
    put(0, ctx.height - 1, string.rep("=", ctx.width), "syntax.type")
    put(left + 1, top - 1, string.rep("_", w - 2), "grey")
    for y = top, ctx.height - 2 do
      put(left, y, "|", "cyan")
      put(left + w - 1, y, "|", "cyan")
    end
    -- each fly wanders a little and shows as a dot, an o or a star as it pulses
    for _, fly in ipairs(flies) do
      local wander = 0.2 * ctx.speed * ctx.dt
      fly.x = math.min(1, math.max(0, fly.x + (ctx.random() - 0.5) * wander))
      fly.y = math.min(1, math.max(0, fly.y + (ctx.random() - 0.5) * wander))
      local level = math.sin(ctx.time * 2 * math.pi / PULSE_PERIOD * ctx.pace + fly.phase)
      if level > -0.3 then
        local ch = "."
        local role = "yellow"
        if level > 0.7 then
          ch = "*"
          role = "bright_yellow"
        elseif level > 0.2 then
          ch = "o"
        end
        local x = left + 1 + math.floor(fly.x * (w - 3))
        local y = top + math.floor(fly.y * (h - 2))
        put(x, y, ch, role)
      end
    end
  end,

  on_recovered = function(ctx)
    return "Fixed. The jar glows again."
  end,
}
```

Save it as `jar.lua` in the scenes folder and run `zacess-scenes jar`.
