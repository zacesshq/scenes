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
- `n` shows the next scene and `b` (or `N`) the one before. The new scene's name and summary show
  on the bottom row for two and a half seconds, then dim and go like the help line. A scene changes
  at most once every 0.4 seconds however fast the key is pressed or held, so the picture is never
  replaced more than three times a second.
- `a` turns the background sound on or off and `[` and `]` change its volume in every mode. A scene
  run on its own has no tones, so `m`, `+` and `-` work the background sound there too. Each
  change is written on the bottom row for two and a half seconds.
- The watch mode adds `1` to `4` to switch bacon's job, `r` to run it again, `p` to pause the
  animation, `s` to turn sound on or off, `n` and `b` to change the scene, `m` to change the sound
  set and `+` and `-` to change the volume (`=` and `_` work too, so Shift is never needed). The
  follow mode has the same keys apart from `1` to `4` and `r`, since it runs nothing itself. Their
  key hints show, dim and go in the same way as the help line, above a status bar that stays on
  screen. A change of scene, sound set or volume is written in the status bar for three seconds.

## Before and after the scene

- A plain line is printed before the program takes over the screen, naming the scene, the theme,
  whether reduced motion is on and the keys that quit, so a screen reader hears what started.
- The scene is then drawn on the terminal's alternate screen with the cursor hidden and the
  terminal in raw mode. When the program ends after a quit key, `Ctrl+C`, the end of
  `--duration`, a panic anywhere in the code or, on Linux and macOS, a `SIGTERM`, `SIGHUP`,
  `SIGQUIT` or `SIGINT` (a closed tmux pane, `kill` or `timeout`), it leaves the alternate screen,
  shows the cursor, turns raw mode off and prints `Zacess Scenes stopped.`, so the shell comes back
  as it was. When the terminal itself has gone, as with a closed tmux pane, the program ends
  within a second of the signal rather than running on unseen. A `SIGKILL` cannot be caught by any program; after one, `reset` gives the terminal
  back. On Windows, closing the console window ends the console along with the program.
- `--duration <minutes>` stops the scene on its own, for a screen that should not run forever.
- The watch and follow modes print the same kind of line before they start: the project or the
  shell followed, the job or command, the theme, whether reduced motion is on and every key. When
  no project is found the watch mode prints a plain message instead, naming the kinds of project
  it knows and `--cmd`. It never takes over the screen then. When bacon's job is asked for without
  bacon the message gives the command that installs it.

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
- The motion level, `calm`, `standard` or `lively`, is set with `--motion`, `ZACESS_MOTION` or
  the config file. Calm moves at 0.7 times the standard speed with fewer things on screen. Lively
  moves at 1.35 times with more, but anything that changes in place (a pulse, a flicker, a wing
  beat, a window light) keeps the standard pace or slower: `Motion::pace` never goes above it. The
  flash checks run at every level.
- Reduced motion can also be turned on in the config file. Any of the sources turns it on; none
  of them can turn the system setting off.
- With reduced motion on, the frame rate is capped at 12 a second, every movement runs at half
  speed (sways, wobbles, drifts and the aurora's breathing included), the rain has no lightning,
  each row of the campfire changes once every 1.5 seconds instead of every 0.6 and stars and
  fireflies take at least five seconds per cycle. Shooting stars are rarer and slower and code rain
  falls at most three rows a second with its characters never swapped.
- A resize is acted on once the terminal has kept one size for 0.15 seconds, so dragging a window
  edge leaves the picture still rather than redrawing it at every step.

## Flashing

Nothing in any scene flashes: no scene changes the whole screen or a large area at once.

- A flash meter in `src/flash.rs` splits a frame into twelve regions plus the whole screen and
  works out each one's brightness from how much of each cell its character covers and how far its
  colour is from the background. A change of a tenth of the most a region can change, between two
  frames, counts as a large change. `check_flashes` runs every built-in scene for a simulated
  minute at two sizes, at all six combinations of motion level and reduced motion, in the
  brightest and darkest themes. It fails if any second has more than three large changes. Two
  opposing changes make one flash, so that is at most one and a half flashes a second, half the
  three a second WCAG 2.3.1 allows. A test proves the meter catches a flashing screen and a
  flashing corner and ignores a small moving shape.
- The same meter runs live on Lua scenes, holding back any frame that would break the limit, and
  on the frames `snapshot --frames` writes.
- Things that change in place keep to the limits below, which are constants in the code with
  tests that hold them:

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
| watch mode | The recovered opening: the pass scene opens out from the middle over the fail scene | Once per fix, over 2 seconds (3.5 under reduced motion, with no sparkles); each cell changes once as the edge passes | `the_opening_never_flashes_and_ends_on_the_new_scene` |
| every scene | A change of scene with `n` or `b` | At most once every 0.4 seconds | `SCENE_DEBOUNCE` in `src/app.rs` and `src/watch/mod.rs` |
| every scene | A change of scene on a timer, with `--shuffle`, `--every` or the config's interval | Every few minutes, 5 by default, never more often than every 3 seconds however small the time asked for | `the_timer_never_changes_scenes_faster_than_the_floor_and_always_moves` |
| watch mode with a command | The change between passing and failing | Once per finished run. Files are looked at every 0.7 seconds and a run starts only once they have been still for 0.3 seconds | `a_run_passes_fails_and_reruns_on_a_change` |
| follow mode | The change between passing, failing and a typo | Once per command, never more often than every 0.4 seconds. Commands that finish closer together than that show only the latest | `following_reads_only_new_commands_and_skips_interruptions` |
| Lua scenes | Anything the script draws | A frame that would make a fourth large change in a second is held back; a scene held back more than ten times in ten seconds is stopped | `a_flashing_script_is_damped_then_stopped` |
| koi-pond | A ripple's ring, which widens in four stages | One stage every 0.6 seconds, every 1.2 under reduced motion | `ripples_widen_slowly_and_tails_sway_slowly` |
| koi-pond | A koi's tail, spread and then closed | One change every 1.2 seconds; still under reduced motion | `ripples_widen_slowly_and_tails_sway_slowly` |
| harbour | A lamp's reflection on the water | One change every 0.8 seconds, every 1.6 under reduced motion | `the_shimmer_and_the_sway_stay_slow` |
| harbour | A mast top, through four positions | One sway takes 4 seconds, so about one change a second; still under reduced motion | `the_shimmer_and_the_sway_stay_slow` |
| lanterns | A lantern's sway, one column either way | One sway takes 7 seconds, 14 under reduced motion | `lanterns_sway_slowly_and_rise` |
| countryside | A puff of smoke, through three sizes | One size a second, one every two seconds under reduced motion | `the_land_slides_past_but_the_train_stays_in_view` |
| winter-cabin | A puff of smoke, through three sizes | One size every 1.2 seconds, every 2.4 under reduced motion | `the_cabin_window_is_lit_and_smoke_rises` |
| autumn | A falling leaf's turn, through four positions | One turn every 0.6 seconds; no turning under reduced motion | `leaves_turn_slowly_and_the_pile_stays_low` |
| blossom | A falling petal's turn and the bird's head | A petal every 0.7 seconds and the bird every 6 seconds; no turning petals under reduced motion | `petals_turn_slowly_and_the_bird_looks_rarely` |
| city-night | One window lighting up or going dark | One window every 1.2 seconds, every 2.4 under reduced motion | `one_window_changes_at_a_time_and_never_quickly` |
| constellations | A figure's lines changing colour as they fade | One step every 1.2 seconds, every 2.4 under reduced motion | `figures_are_drawn_named_and_faded_one_at_a_time` |
| tea | A puff of steam's curl, `(` or `)` | Twice every 2.4 seconds, every 4.8 under reduced motion | `steam_rises_from_the_mug_and_sways_slowly` |
| cat-nap | The cat's back as it breathes and its tail | The back twice every 4 seconds, the tail twice every 7, both half as often under reduced motion | `the_cat_breathes_slowly_and_its_tail_stirs_rarely` |
| garden | A stem growing a row, a bud opening and a butterfly's wings | A stem row at most once a second, wings twice every 1.2 seconds, half as often under reduced motion | `stems_grow_slowly_and_wings_beat_slowly` |
| tiles | One tile taking a new colour or settling to its outline | One tile every 3 seconds on a small grid and never more often than every 0.7 seconds, half as often under reduced motion | `one_tile_changes_at_a_time_and_never_quickly` |
| flood | The water's surface row as it swells | At most twice every 12 seconds, a third of a row each way; the rise takes 45 seconds | `the_surface_row_moves_at_most_twice_a_swell` |
| listing-ship | The ship's angle as it lists and rolls | Under two degrees a second, so the bow moves under a row a second; a roll takes 11 seconds | `the_ship_lists_slowly_then_rolls_gently_at_its_angle` |
| corrosion | A rusty cell darkening | One stage every 8 seconds, four stages in its life | `a_cell_darkens_no_faster_than_one_stage_a_period` |
| tangle | The cables as they tangle | The tangle grows by at most about a thirtieth of the whole a second | `the_tangle_grows_slowly` |
| wilt | The flower heads and leaves drying | Each changes once or twice over 50 seconds; the stems sway once every 9 seconds | `stems_droop_leaves_fall_and_land_on_the_sill` |
| fog | The fog thickening over a cell | The bank covers at most about a tenth of the screen a second; the texture drifts 1.2 columns a second over patches 9 columns wide | `the_fog_rolls_in_slowly_and_hides_the_signs`, `the_noise_is_smooth_and_in_range` |
| broken-bridge | A splash where a fragment lands, `o` then `( )` | One fragment every 4 seconds (`FRAGMENT_PERIOD`), the splash cell changing twice in 0.7 seconds; half as often under reduced motion | `fragments_fall_slowly_and_splash_once_each` |
| leak | A drip's splash beside the water's surface and the bucket's water level | One drip every 1.1 seconds (`DRIP_PERIOD`), the splash showing for 0.3 seconds; the water rises one row every few drips | `drips_are_slow_and_never_quicker_on_lively` |
| blackout | A window going dark, then perhaps a candle | One window at a time, at least 0.25 seconds apart (`MIN_WINDOW_GAP`); each window changes at most twice, five seconds apart | `no_cell_changes_more_than_once_a_second_while_the_lights_go_out` |
| traffic-jam | An exhaust puff fading from `o` to `.` | One puff a car every 3 seconds (`PUFF_PERIOD`), changing once in its 2.4 second life | `puffs_are_slow_on_every_level` |
| erosion | A piece of the face breaking away and a splash | One piece every 2.2 seconds (`CRUMBLE_PERIOD`), then one pebble every 7 seconds; the splash cell changes twice in 0.7 seconds | `pieces_break_away_slowly_on_every_level` |
| drought | A plate edge of cracks appearing and the puddle losing a cell | Cracks spread over 60 seconds (`CRACK_SECONDS`), each cell appearing once; the puddle loses one cell every 6 seconds (`PUDDLE_SHRINK`) | `the_puddle_shrinks_one_cell_at_a_time_at_most_every_few_seconds` |

Things that move, such as drops, fish, flakes and fireflies, change the cells they pass through as
they go. Counted cell by cell over 30 seconds at 20 by 8, 80 by 24 and 120 by 40, the most changes
any one cell saw in a second were: rain 12 (9 under reduced motion), code rain 18 (3), sea 8 (5),
snow, starfield, fireflies and aurora 7 (4 to 6), campfire 6 with its embers (4), waves 5 (3) and
zacess 3 (2). These are small shapes moving across the screen, not flashes. Under reduced
motion every one of them moves at half speed. The scenes added in 0.3.0 are held by the region
check above rather than counted cell by cell.

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
- Lua scenes cannot name a colour. They draw with a palette role such as `text`, `accent`,
  `error` or `syntax.keyword`, which the program turns into the theme's colour, falling back to
  the text colour where a role is below the floor. `every_role_keeps_the_floor_in_every_theme`
  holds it.
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

A scene on its own is silent unless the background sound is turned on. The watch and follow modes
can play seven short events in one of eight sound sets: `bells`, `marimba`, `kalimba`, `pad`,
`chip`, `glass`, `piano` and `rain`.

| Event | When | On by default |
| --- | --- | --- |
| pass | A run passes after waiting or a key asked for it | Yes |
| fail | A run fails after passing, on the first result or when a key asked for it | Yes |
| tick | A rebuild starts, at 40 percent of the level | Yes |
| recovered | A run passes after failing | Yes |
| warnings | A passing run has more warnings than the last | Yes |
| still_failing | A run fails again, at 30 percent of the level | No |
| typo | A followed command was not found, at 60 percent of the level | Yes |

- Sound is never the only signal. Every event has the same state on screen at the same moment: the
  banner, the scene and the status bar show passing, failing or recovered. The status bar shows
  `running` while a rebuild runs and the warning count.
- Every set plays the same phrase for each event, a rising triad for a pass, a falling fourth for a
  failure, the triad landing an octave up for a fix, a rising minor third for warnings and two soft
  low notes a step apart for a typo, so the meaning does not change with the timbre. A typo shows
  TYPO on screen as well as its tone.
- Every tone is levelled to the same loudness, measured as the root mean square of its loudest 50
  milliseconds. It is kept under a peak of 70 percent of full scale. `every_set_and_event_is_levelled_to_the_same_loudness`
  holds each tone within a decibel of its level and every set's pass tone within 1.5 decibels of
  the others. No partial above 6 kHz is synthesised, so the 8-bit set stays soft.
- Sound is turned off with `--no-sound`, with `ZACESS_SOUND=0` (or `false`, `no` or `off`), in the
  config file or with `s` while it runs. The status bar then says `sound off`. Each event can be
  turned off in the config. The volume is set from 0 to 100 with `--volume`, `ZACESS_VOLUME`, the
  config or `+` and `-`, 50 by default. A volume of 0 is silent.
- Quiet hours in the config, such as `22:00-07:00`, keep every event silent between two times of
  day. The status bar says `quiet hours` while they apply.
- The tones are synthesised with soft envelopes and short fades at both ends, so they never click.
  `no_sample_reaches_full_scale_at_any_volume` and
  `tones_start_and_end_in_silence_so_they_never_click` check every set and event.
- The longest tone, the pad set's recovered phrase, lasts one and a quarter seconds.
- Without a system player the watch mode stays silent and the status bar says `no sound player`.

### Background sound

- Off unless turned on with `--ambient`, `ZACESS_AMBIENT=1` or `enabled = true` under `[ambient]`.
  `--no-ambient` or `a` turns it off again.
- A quiet loop that suits the scene: water, rain, wind, fire or a night meadow. It carries no
  meaning and nothing depends on hearing it.
- It plays at 20 out of 100 by default, under the tones' 50. It is also levelled to about a third
  of the tones' loudness at the same volume. `no_sample_reaches_full_scale_and_it_stays_under_the_tones`
  checks every loop. `[` and `]` change it in steps of 10 and 0 is silent.
- It stops for a moment while a tone plays, so a pass, a failure or a typo is always heard over it.
- It pauses with the scene, keeps to quiet hours and is off whenever sound is turned off with
  `--no-sound`, `ZACESS_SOUND=0` or the config.
- Each loop lasts 20 seconds and its end is crossfaded into its start, so it never clicks at the
  join (`the_loop_joins_without_a_jump`). The player is started again as a loop ends, which leaves
  a gap of a few milliseconds.

## Notifications

`--notify`, `ZACESS_NOTIFY=1` or `enabled = true` under `[notifications]` in the config sends a
desktop notification when a build starts failing and when it is fixed, such as `my-project: check
failed with 2 problems.` It uses `osascript` on macOS, `notify-send` on Linux and a PowerShell
toast on Windows, with a tray balloon where the toast is not available. VoiceOver, Orca and
Narrator announce notifications, so the state can be heard without looking at the terminal. They
are off by default because a notification interrupts.

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
- The watch mode's status changes happen on the alternate screen, so a screen reader does not read
  them there. With `--notify` each change between passing and failing is also sent as a desktop
  notification, which screen readers announce. Without it the tones are the only change heard
  without looking.
- A Lua scene is checked for flashing while it runs, not before, so its first second can hold up
  to three large changes before frames are held back. Built-in scenes are checked in the tests.
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
