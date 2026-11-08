# Snapshots

`zacess-scenes snapshot` draws a scene with the real renderer and no terminal:

```sh
zacess-scenes snapshot koi-pond --out koi-pond.svg                  # an SVG still
zacess-scenes snapshot storm --state fail --out watch-fail.svg      # the watch mode, sample data
zacess-scenes snapshot lanterns --frames 72 --fps 12 --out a.json   # JSON frames
```

A still is a self-contained SVG inside a rounded terminal window with the scene and theme as its
title, on the theme's background. Pixels are drawn as rectangles on a grid of half rows: neighbours
of one colour on a half row share a rectangle and a rectangle carries on down through every half
row with the same run, so a solid shape is one rectangle rather than a stack of slices whose edges
a renderer would smooth into faint lines between rows. Text, such as the watch mode's banner, is
monospace text in its palette colour over them. A still has `role="img"` and an `aria-label` that
names the scene and the theme and gives the scene's summary. The gallery stills at 80 by 24 are
about 15 to 120 KB, most of them 20 to 60 KB. `--theme`, `--size` (80x24 by default), `--time` (how many seconds
in, 6 by default), `--seed`, `--motion` and `--reduced-motion` set what is drawn. `--clock` sets
the local time a Lua scene's `ctx.clock` starts at, as `HH:MM` or `YYYY-MM-DDTHH:MM`; without it
the clock starts at Monday 1 June 2026 at 09:00 and moves on with `--time`, so a still never
depends on when it was taken. `--state pass` or
`--state fail` adds the watch mode's banner, caption and status bar with sample data. Lua scenes
work too. With a state, a Lua scene's `on_pass` or `on_fail` hook runs as the scene starts, as it
does when a run finishes in the watch mode, so the still shows what the scene does at that moment
and the caption the hook returns.

With `--frames <n>` the output is JSON instead, for a web page to play in a canvas or a `pre`
element:

```json
{"width":80,"height":24,"fps":12,"background":"#000000","frames":[[{"x":3,"y":0,"text":"~~ ~","colour":"#5fd7ff"}]]}
```

Each frame is a list of runs of one colour, drawn in order: first every cell background other
than the theme's as full blocks (`█`), then every glyph over them. A pixel cell is its top pixel
as the background and its bottom pixel as a lower half block (`▄`), so a page that draws runs of
coloured text plays the pixel scenes exactly. Each lower half block covers the line between its
row and the next, so a page that grows every block a little to close the gaps shows no seams
between rows. Spaces in a run draw nothing, so a run of one colour carries on across a short gap
rather than starting again. The frames go through the flash
meter, so a page playing them never flashes. [frames](frames) holds frames for eight scenes
(`koi-pond`, `lanterns`, `tea`, `constellations`, `fog`, `traffic-jam`, `drought` and `zacess`),
six seconds each at 6 frames a second, which keeps each file between about 0.7 and 1.1 MB.
