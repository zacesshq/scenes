# Accessibility

Zacess Scenes is a Zacess product, so it is built for people with low vision, light sensitivity
and motion sensitivity first. This statement describes what the program does today, measured from
its code and its tests, not what it aims to do.

## Keys and the help line

- `Space` pauses and resumes. `h` or `?` shows the help line again. `q`, `Esc` and `Ctrl+C` quit.
- When a scene starts, the bottom row shows the keys in the theme's text colour for four seconds,
  then in the muted colour for one and a half seconds, then not at all. It never blinks. The text
  is shortened on narrow terminals so it always fits, down to 20 columns.
- While a scene is paused the bottom row says `Paused` and how to resume, for as long as it is
  paused, down to `Paused: Space` at 20 columns. Pause toggles at most once every 0.3 seconds and
  key repeats are ignored, so holding `Space` never makes the line flash.
- The mouse is not used and nothing needs a modifier key apart from `Ctrl+C`, which the terminal
  passes on as a key.
- The watch mode adds `1` to `4` to switch the job, `r` to run it again, `p` to pause the animation
  and `s` to turn sound on or off. Its key hint shows, dims and goes in the same way as the help
  line, above a status bar that stays on screen.

## Before and after the scene

- A plain line is printed before the program takes over the screen, naming the scene, the theme,
  whether reduced motion is on and the keys that quit, so a screen reader hears what started.
- The scene is then drawn on the terminal's alternate screen with the cursor hidden and the
  terminal in raw mode. When the program ends after a quit key, `Ctrl+C`, the end of
  `--duration`, a panic anywhere in the code or, on Linux and macOS, a `SIGTERM`, `SIGHUP`,
  `SIGQUIT` or `SIGINT` (a closed tmux pane, `kill` or `timeout`), it leaves the alternate screen,
  shows the cursor, turns raw mode off and prints `Zacess Scenes stopped.`, so the shell comes back
  as it was. A `SIGKILL` cannot be caught by any program; after one, `reset` gives the terminal
  back. On Windows, closing the console window ends the console along with the program.
- `--duration <minutes>` stops the scene on its own, for a screen that should not run forever.
- The watch mode prints the same kind of line before it starts: the project, the job, the theme,
  whether reduced motion is on and every key. Without bacon or outside a Cargo project it prints
  a plain message instead and never takes over the screen. Without bacon the message gives the
  command that installs it.

## Motion

- The default frame rate is 20 frames a second and `--fps` cannot go above 30. `--slow` halves the
  speed of everything.
- Reduced motion is on when any of these holds: the `--reduced-motion` flag; the environment
  variable `ZACESS_REDUCED_MOTION` set to `1`, `true`, `yes` or `on`; the system setting, read
  without any extra dependency. The system setting is Reduce Motion on macOS (`defaults read
  com.apple.universalaccess reduceMotion`), animations turned off in GNOME (`gsettings get
  org.gnome.desktop.interface enable-animations`) or animations turned off in the Windows visual
  effects (the Animation effects switch, stored in `UserPreferencesMask` under
  `Control Panel\Desktop` or the `MinAnimate` value under
  `Control Panel\Desktop\WindowMetrics`). A headless render never asks the system, so it prints
  the same frame everywhere.
- With reduced motion on, the frame rate is capped at 12 a second, every movement runs at half
  speed (sways, wobbles, drifts and the aurora's breathing included), the rain has no lightning,
  each row of the campfire changes once every 1.5 seconds instead of every 0.6 and stars and
  fireflies take at least five seconds per cycle. Shooting stars are rarer and slower and code rain
  falls at most three rows a second with its characters never swapped.
- A resize is acted on once the terminal has kept one size for 0.15 seconds, so dragging a window
  edge leaves the picture still rather than redrawing it at every step.

## Flashing

Nothing in any scene flashes: no scene changes the whole screen or a large area at once. Things
that change in place keep to the limits below, which are constants in the code with tests that
hold them:

| Scene | What changes in place | How often at most | Test |
| --- | --- | --- | --- |
| rain | A bolt of lightning, drawn as one jagged line in the upper third, never the whole screen | Once every 5 seconds since the last, whatever resizes happen, on screen for 0.12 seconds, never under reduced motion | `lightning_keeps_at_least_five_seconds_between_flashes`, `resizing_never_brings_lightning_early`, `lightning_never_strikes_under_reduced_motion` |
| campfire | A row of the flame, which takes a new lean and width; rows change on staggered timers | Each row every 0.6 seconds, so no flame cell more than twice a second; every 1.5 seconds under reduced motion, so a cell at most once a second | `no_flame_cell_changes_more_than_twice_a_second` |
| starfield | A star's brightness, through three characters | One cycle takes at least 2.5 seconds, at least 5 under reduced motion | `stars_twinkle_slowly_and_the_moon_hangs_top_right` |
| fireflies | A light's brightness, through three characters | One cycle takes at least 2.5 seconds, at least 5 under reduced motion | `lights_pulse_slowly_over_a_grassy_bottom_row` |
| waves | A gull's wings, through four positions | One beat takes 1.6 seconds, 3.2 under reduced motion | `the_sea_fills_below_the_horizon_and_gulls_fly_above_it` |
| code-rain | A cell of a column's trail, as the column moves down | At most three times a second under reduced motion | `calm_trails_change_a_cell_at_most_three_times_a_second` |
| every scene | The help line | Fades in two steps, over 5.5 seconds, never blinks | `the_overlay_shows_then_dims_then_goes` |
| watch mode storm | A bolt of lightning, as in the rain | Once every 5 seconds since the last, the first one 5 seconds after the storm appears, never under reduced motion | `the_storm_keeps_the_lightning_limits` |
| watch mode | The change between passing and failing: the scene, the banner and the list | Once per finished run of the job, in one step with no fade, flash or inverted frame. A run takes as long as the build, so seconds apart at the least | `the_state_moves_from_waiting_to_passing_to_failing_and_back` |
| watch mode | The key hint | Fades in two steps, over 5.5 seconds, never blinks | `the_hint_shows_then_dims_then_goes` |

Things that move, such as drops, fish, flakes and fireflies, change the cells they pass through as
they go. Counted cell by cell over 30 seconds at 20 by 8, 80 by 24 and 120 by 40, the most changes
any one cell saw in a second were: rain 12 (9 under reduced motion), code rain 18 (3), sea 8 (5),
snow, starfield, fireflies and aurora 7 (4 to 6), campfire 6 with its embers (4), waves 5 (3) and
zacess 3 (2). These are small shapes moving across the screen, not flashes. Under reduced
motion every one of them moves at half speed.

## Colour and contrast

- Every colour comes from the Zacess palette, version 0.5.0: the theme's text, muted and accent
  colours, its syntax colours and its sixteen terminal colours. ANSI black is never used because in
  Zacess Deep Contrast Dark it is the background itself.
- Every colour a scene draws keeps at least 4.5 to 1 contrast on the theme's background, the WCAG AA
  floor for text. It keeps at least 7 to 1, the AAA level, on the four High Contrast and Deep
  Contrast themes. On those themes a terminal colour below 7 to 1 is swapped for the palette colour
  nearest in hue that keeps it. The ratio is computed by `Rgb::contrast` in `src/colour.rs` and
  three tests hold the floors: `every_drawable_colour_keeps_the_contrast_floor_in_every_theme`
  checks every colour a theme offers; `every_colour_role_keeps_the_theme_floor` checks every role
  a scene draws with; each scene's `passes_the_shared_checks` draws the scene in all eight themes
  and checks every colour of every frame over a simulated minute at 80 by 24, long enough to
  include lightning, shooting stars and the whale, plus sampled frames at 20 by 8 and 120 by 40.
- The light shade `░` covers about a quarter of its cell, so it reads fainter than its measured
  colour. It is only the outer edge of an aurora band or a flame. On the High Contrast and Deep
  Contrast themes the medium shade `▒` is drawn there instead.
- Where two colours touch inside one shape, the Zacess mark's band and its page or tile, a row of
  background keeps them apart, so the band never depends on the contrast between the two colours.
  `the_band_never_touches_a_cell_of_another_colour` checks it in every theme.
- The watch mode's banner, list and status bar use the theme's green for passing and its error
  and warning colours for problems. Where the error or warning colour is below the theme's floor
  its red or yellow stands in. `every_overlay_colour_keeps_the_floor_in_every_theme` draws every
  state in all eight themes and checks every colour. It also checks that the error colour differs
  from both the warning and the passing colours.
- Colour is never the only signal. Fish of different kinds are different shapes, flame layers use
  different shade characters, a star's brightness is a different character and the settled snow is
  a solid block. In the watch mode passing reads `✓ passing` with ALL GOOD, failing reads
`✗ failing` with BUILD FAILED, errors are marked with a cross and warnings with `!` as well as
their own words. With `NO_COLOR` set, every character is drawn in the terminal's own text colour and
  every scene stays readable.
- The program sets no background colour of its own. It assumes the terminal is already on the same
  Zacess theme. On a terminal with a different background the measured ratios do not apply.
- On a light theme the snow bank is drawn in the theme's grey rather than solid black and the
  Zacess mark is drawn as its dark tile with the page left as the background, so it keeps the look
  of the real mark.

## Sound

The scenes make no sound. The watch mode can play three short tones: a gentle rising chime when a
run passes, a lower two-note tone when it fails and a soft tick when a rebuild starts.

- Sound is never the only signal. Every tone has the same state on screen at the same moment: the
  banner, the scene and the status bar show passing or failing and the status bar shows `running`
  while a rebuild runs.
- The pass and fail tones play once when the result changes or when a key asked for the run, not on
  every save, so a build that keeps failing stays quiet.
- Sound is turned off with `--no-sound`, with `ZACESS_SOUND=0` (or `false`, `no` or `off`) or with
  `s` while it runs. The status bar then says `sound off`. The volume is set from 0 to 100 with
  `--volume` or `ZACESS_VOLUME`, 50 by default. A volume of 0 is silent.
- The tones are synthesised by the program with soft envelopes and short fades at both ends, so they
  never click. The loudest sample sits at 70 percent of full scale at volume 100 and never clips.
  `no_sample_reaches_full_scale_at_any_volume` and
  `tones_start_and_end_in_silence_so_they_never_click` hold this.
- The longest tone, the pass chime, lasts just over a second.
- Without a system player the watch mode stays silent and the status bar says `no sound player`.

## Size

Every scene is laid out for the terminal it is in, from 20 columns by 8 rows upwards. It lays
itself out again when the terminal is resized while it runs, keeping what it can, such as the
settled snow and the flame. A terminal that reports a size of zero is treated as 80 by 24. Things
that need room, such as the
whale, the moon and the sun, are left out on a terminal too small for them rather than squeezed in.

## Known limits

> [!IMPORTANT]
> The scenes use block and shade characters (`█ ▓ ▒ ░ ▄ ◣ ◥`). A terminal font without them shows
> boxes or gaps in the aurora, the campfire, the snow bank and the Zacess mark. Every other
> character is plain ASCII. On the Linux console (`TERM=linux`) and the classic Windows console
> (no `WT_SESSION`), whose fonts lack `◣` and `◥`, the mark's fold is drawn as a square cut corner.

- Colours are sent as exact 24 bit colour only when the terminal says it supports it. Elsewhere,
  including Terminal.app before macOS 26, the Linux console, tmux or screen without true colour and
  the classic Windows console, each colour is sent as the nearest entry in the 256 colour table
  that still keeps the floor. `--colour truecolor` or `--colour 256` overrides the guess. The
  Linux console shows only sixteen colours, so the ratios there are approximate.
- Block and shade characters are ambiguous width in Unicode. A terminal set to draw ambiguous
  characters two columns wide spreads them out, but every cell after one is positioned explicitly,
  so the picture never drifts or wraps.
- The alternate screen is not read by screen readers. What a screen reader user gets is the plain
  line before the scene and the line after it.
- The scenes make no sound and nothing gives haptic feedback. The watch mode's tones are described
  under [Sound](#sound).
- The watch mode's status changes happen on the alternate screen, so a screen reader does not hear
  them. Its tones are the only change heard without looking.
- The system reduced motion setting is read on macOS, GNOME and Windows (both the Animation
  effects switch in Settings and the minimise animation in the performance options). Another
  desktop needs the flag or the environment variable. A settings command that takes longer than
  half a second, such as `gsettings` without a session bus, counts as not set.

Many of these are preferences: the frame rate, the speed, the theme and the scene are all flags.
If a different default would help others, open an issue or a pull request.

## Reporting a barrier

An accessibility problem is a bug. Please open an issue with the
[accessibility problem
form](https://github.com/zacesshq/scenes/issues/new?template=accessibility_problem.yml),
describing what you were doing and what got in the way. You never need to share a diagnosis or
anything personal. To report privately, email [contact@zacess.com](mailto:contact@zacess.com).

## The shared statement

The organisation's [accessibility
statement](https://github.com/zacesshq/.github/blob/main/ACCESSIBILITY.md)
covers every Zacess repository. This file takes precedence where the two differ.
