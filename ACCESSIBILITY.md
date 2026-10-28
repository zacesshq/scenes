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
  paused.
- The mouse is not used and nothing needs a modifier key apart from `Ctrl+C`, which the terminal
  passes on as a key.

## Before and after the scene

- A plain line is printed before the program takes over the screen, naming the scene, the theme,
  whether reduced motion is on and the keys that quit, so a screen reader hears what started.
- The scene is then drawn on the terminal's alternate screen with the cursor hidden and the
  terminal in raw mode. When the program ends, after a quit, a panic anywhere in the code or
  `Ctrl+C`, it leaves the alternate screen, shows the cursor, turns raw mode off and prints
  `Zacess Scenes stopped.` so the shell comes back exactly as it was.
- `--duration <minutes>` stops the scene on its own, for a screen that should not run forever.

## Motion

- The default frame rate is 20 frames a second and `--fps` cannot go above 30. `--slow` halves the
  speed of everything.
- Reduced motion is on when any of these holds: the `--reduced-motion` flag; the environment
  variable `ZACESS_REDUCED_MOTION` set to `1`, `true`, `yes` or `on`; the system setting, read
  without any extra dependency. The system setting is Reduce Motion on macOS (`defaults read
  com.apple.universalaccess reduceMotion`), animations turned off in GNOME (`gsettings get
  org.gnome.desktop.interface enable-animations`) or animations turned off in the Windows visual
  effects (the `MinAnimate` value under `Control Panel\Desktop\WindowMetrics`). A headless render
  never asks the system, so it prints the same frame everywhere.
- With reduced motion on, the frame rate is capped at 12 a second, every scene moves at half speed,
  the rain has no lightning, the campfire takes a new shape once a second instead of just under
  three times a second, stars and fireflies take at least five seconds per cycle, shooting stars
  are rarer and slower and the characters in code rain never change while they fall.

## Flashing

Nothing in any scene flashes and nothing changes more than three times a second. The limits are
constants in the code with tests that hold them:

| Scene | What changes | How often at most | Test |
| --- | --- | --- | --- |
| rain | A bolt of lightning, drawn as one jagged line in the upper third, never the whole screen | Once every 5 seconds, on screen for 0.12 seconds, never under reduced motion | `lightning_keeps_at_least_five_seconds_between_flashes`, `lightning_never_strikes_under_reduced_motion` |
| campfire | The flame's shape, which blends smoothly from one shape to the next | A new shape every 0.34 seconds, every second under reduced motion | `the_flame_changes_shape_no_more_than_three_times_a_second` |
| starfield | A star's brightness, through three characters | One cycle takes at least 2.5 seconds, at least 5 under reduced motion | `stars_twinkle_slowly_and_the_moon_hangs_top_right` |
| fireflies | A light's brightness, through three characters | One cycle takes at least 2.5 seconds, at least 5 under reduced motion | `lights_pulse_slowly_over_a_grassy_bottom_row` |
| waves | A gull's wings, through four positions | One beat takes 1.6 seconds, 3.2 under reduced motion | `the_sea_fills_below_the_horizon_and_gulls_fly_above_it` |
| every scene | The help line | Fades in two steps, over 5.5 seconds, never blinks | `the_overlay_shows_then_dims_then_goes` |

## Colour and contrast

- Every colour comes from the Zacess palette, version 0.4.0: the theme's text, muted and accent
  colours, its syntax colours and its sixteen terminal colours. ANSI black is never used because in
  Zacess Deep Contrast Dark it is the background itself.
- Every colour a scene can draw keeps at least 4.5 to 1 contrast on the theme's background, the
  WCAG AA floor for text. The ratio is computed by `Rgb::contrast` in `src/colour.rs` and two tests
  hold the floor: `every_drawable_colour_keeps_the_contrast_floor_in_every_theme` checks every
  colour a theme offers; each scene's `passes_the_shared_checks` draws the scene in all eight
  themes at 20 by 8, 80 by 24 and 120 by 40 and checks every colour in every frame.
- Colour is never the only signal. Fish of different kinds are different shapes, flame layers use
  different shade characters, a star's brightness is a different character and the settled snow is
  a solid block. With `NO_COLOR` set, every character is drawn in the terminal's own text colour and
  every scene stays readable.
- The program sets no background colour of its own. It assumes the terminal is already on the same
  Zacess theme, which is how the Zacess theme switcher sets things up. On a terminal with a
  different background the measured ratios do not apply.
- On a light theme the snow bank is drawn in the theme's grey rather than solid black and the
  Zacess mark is drawn as its dark tile with the page left as the background, so it keeps the look
  of the real mark.

## Size

Every scene is laid out for the terminal it is in, from 20 columns by 8 rows upwards. It lays
itself out again when the terminal is resized while it runs. Things that need room, such as the
whale, the moon and the sun, are left out on a terminal too small for them rather than squeezed in.

## Known limits

> [!IMPORTANT]
> The scenes use block and shade characters (`█ ▓ ▒ ░ ▄ ◣ ◥`). A terminal font without them shows
> boxes or gaps in the aurora, the campfire, the snow bank and the Zacess mark. Every other
> character is plain ASCII.

- The colours are sent as 24 bit colour, so the terminal needs true colour support. Nearly every
  current terminal has it; the Windows legacy console does not, so use Windows Terminal there.
- The alternate screen is not read by screen readers. What a screen reader user gets is the plain
  line before the scene and the line after it.
- There is no sound and no haptic feedback.
- The system reduced motion setting is read on macOS, GNOME and Windows. Another desktop needs the
  flag or the environment variable.

Many of these are preferences: the frame rate, the speed, the theme and the scene are all flags.
If a different default would help others, open an issue or a pull request.

## Reporting a barrier

An accessibility problem is a bug. Please open an issue with the
[accessibility problem form](https://github.com/zacesshq/scenes/issues/new?template=accessibility_problem.yml),
describing what you were doing and what got in the way. You never need to share a diagnosis or
anything personal. To report privately, email [contact@zacess.com](mailto:contact@zacess.com).
