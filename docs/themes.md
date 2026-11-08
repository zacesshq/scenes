# Themes

The colours are the terminal and syntax colours of [Zacess
Themes](https://github.com/zacesshq/themes)
1.0.0, embedded in the program. The theme is picked in this order:

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
| `warm-dark` | `warm-dark` | Zacess Warm Dark |
| `warm-light` | `warm-light` | Zacess Warm Light |

Each theme's contrast floor comes from the palette's `textContrast`: 7 to 1 for the first eight
themes and 4.5 to 1 for Warm Dark and Warm Light, which the palette softens for long sessions.
Every colour a scene draws as text keeps that floor; pixel shapes that carry meaning keep at least
the 3 to 1 non-text floor and background imagery stays below it. A terminal colour below it is
replaced by its bright or plain twin when that keeps the floor. Otherwise it keeps its own hue and
only its lightness moves away from the background until it reaches the floor, so the yellow on
Cream becomes a deep ochre rather than another theme colour. Only a colour that no lightness can
lift to the floor is swapped for the palette colour nearest in hue.

A name is matched without regard to case, the `Zacess` prefix or whether spaces or hyphens
separate the words. The built-in scenes paint their own background in the theme's colours, so
the terminal's own background shows only around text drawn on it. Lua scenes set no background
colour and draw on whatever the terminal already shows, so the terminal should be on the same
theme.

## Colour depth

Colours are sent as exact 24 bit colour when the terminal says it supports it (`COLORTERM` set to
`truecolor` or `24bit`, Windows Terminal, iTerm2, WezTerm, Ghostty and a few others). Anywhere
else, such as Terminal.app before macOS 26, the Linux console or tmux without true colour, each
colour is sent as the nearest entry in the 256 colour table that still keeps the contrast floor.
A pixel keeps no floor, so it is sent as the plain nearest entry. A cell whose two pixels land on
the same entry is sent as a space of that colour.
`--colour truecolor` or `--colour 256` overrides the guess.

With the `NO_COLOR` environment variable set, every character is drawn in the terminal's own text
colour and the scene is still readable, because no scene relies on colour alone. A pixel that
stands out from the background at the non-text floor of 3 to 1 or more becomes a solid half
block and the background imagery below it becomes blank, so shapes keep their outline.
