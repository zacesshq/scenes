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
| mlua | Running Lua scenes of the user's own in a sandbox | MIT |
| lua-src | The Lua 5.4 source, compiled into the binary so nothing needs installing | MIT |

mlua also links a few small crates of its own (bstr, either, memchr, num-traits, parking_lot,
rustc-hash and smallvec, with their own dependencies), each under MIT or Apache-2.0 at the
user's choice; memchr is also offered under the Unlicense.

## Lua

The Lua scenes run on [Lua 5.4](https://www.lua.org/), copyright Lua.org, PUC-Rio, under the
[MIT licence](https://www.lua.org/license.html). It is compiled from source into the binary
through lua-src and reached through mlua. `examples/scenes/lantern.lua` and the starter script
that `zacess-scenes scene new` writes are our own and under this project's MIT licence.

## bacon

The watch mode runs [bacon](https://github.com/Canop/bacon) by Canop, which is licensed under the
[GNU Affero General Public License 3.0](https://github.com/Canop/bacon/blob/main/LICENSE). bacon is
a separate program that the person installs themselves with `cargo install --locked bacon`. It is
not bundled with, linked into or copied into Zacess Scenes. Zacess Scenes starts it as its own
process and reads the JSON report it writes to a temporary file.

## Sounds

The watch mode's six sound sets are synthesised by the program's own code in `src/sound.rs` when
they are first needed. No recorded audio is bundled.

## Scenes and gallery

Every scene, its captions and the stills in `docs/images/scenes` and the frames in `docs/frames`
are our own, drawn by the program itself and covered by this project's MIT licence.

## Methods

The contrast ratios use the relative luminance and contrast formulas from
[WCAG 2.2](https://www.w3.org/TR/WCAG22/).

## Fonts

No font is bundled. The scenes draw in the terminal's own font.
