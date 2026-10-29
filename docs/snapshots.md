# Snapshots

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
playing them never flashes. [frames](frames) holds frames for eight scenes (`koi-pond`, `lanterns`, `tea`, `constellations`, `fog`, `traffic-jam`, `drought` and `zacess`), six seconds
each at 12 frames a second.
