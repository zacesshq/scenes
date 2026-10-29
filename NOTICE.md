# Licensing notice

Zacess Scenes is licensed under the [MIT licence](LICENSE).

## The Zacess mark

The `zacess` scene in `src/scenes/zacess.rs` draws the Zacess mark, a page with a folded corner
and a band across it, from block characters. The MIT licence covers the program and its code. It
does not grant any right to use the Zacess name or mark to suggest that another product is Zacess
or is endorsed by it.

## The palette

`palette/zacess.json` is a copy of the palette from [Zacess Themes](https://github.com/zacesshq/themes),
used under the same MIT licence. `src/palette.rs` is generated from it by
`scripts/build-palette.py`.

## Dependencies

The released binary is built from these crates, each linked in rather than bundled as a file:

| Crate | Used for | Licence |
| --- | --- | --- |
| crossterm | Raw mode, the alternate screen, colours and keys | MIT |
| signal-hook (Unix only) | Restoring the terminal on a termination signal | MIT or Apache-2.0 |

## bacon

The watch mode runs [bacon](https://github.com/Canop/bacon) by Canop, which is licensed under the
[GNU Affero General Public License 3.0](https://github.com/Canop/bacon/blob/main/LICENSE). bacon is
a separate program that the person installs themselves with `cargo install --locked bacon`. It is
not bundled with, linked into or copied into Zacess Scenes. Zacess Scenes starts it as its own
process and reads the JSON report it writes to a temporary file.

## Sounds

The watch mode's tones are synthesised by the program's own code in `src/sound.rs` when they are
first needed. No recorded audio is bundled.

## Methods

The contrast ratios use the relative luminance and contrast formulas from
[WCAG 2.2](https://www.w3.org/TR/WCAG22/).

## Fonts

No font is bundled. The scenes draw in the terminal's own font.
