# Themes

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

## Colour depth

Colours are sent as exact 24 bit colour when the terminal says it supports it (`COLORTERM` set to
`truecolor` or `24bit`, Windows Terminal, iTerm2, WezTerm, Ghostty and a few others). Anywhere
else, such as Terminal.app before macOS 26, the Linux console or tmux without true colour, each
colour is sent as the nearest entry in the 256 colour table that still keeps the contrast floor.
`--colour truecolor` or `--colour 256` overrides the guess.

With the `NO_COLOR` environment variable set, every character is drawn in the terminal's own text
colour and the scene is still readable, because no scene relies on colour alone.
