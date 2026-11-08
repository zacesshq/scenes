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
  moves at 1.35 times with more, while pulses, flickers, wing beats and window lights keep the
  standard pace or slower: they run on `Motion::pace`, which never goes above it. So do the slow
  changes in place, such as how often a frog hops in `lily-pads` and the light shafts in
  `forest-stream`. The flash checks run at every level.
- Reduced motion can also be turned on in the config file. Any of the sources turns it on; none
  of them can turn the system setting off.
- With reduced motion on, the frame rate is capped at 12 a second, every movement runs at half
  speed or slower (sways, wobbles, drifts and the aurora's breathing included), the rain and the
  storm have no lightning, each row of the campfire changes once every 1.5 seconds instead of
  every 0.6 and stars and fireflies take at least five seconds per cycle. Shooting stars are rarer and slower and code rain
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
- The same meter runs live on Lua scenes, holding back any frame that would break the limit. It
  runs on the frames `snapshot --frames` writes too.
- Things that change in place keep to the limits below, which are constants in the code with
  tests that hold them:

| Scene | What changes in place | How often at most | Test |
| --- | --- | --- | --- |
| every scene | The help line | Fades in two steps, over 5.5 seconds, never blinks | `the_overlay_shows_then_dims_then_goes` |
| watch mode | The change between passing and failing: the scene, the banner and the list | Once per finished run of the job, in one step with no fade, flash or inverted frame. A run takes as long as the build, so seconds apart at the least | `the_state_moves_from_waiting_to_passing_to_failing_and_back` |
| watch mode | The key hint | Fades in two steps, over 5.5 seconds, never blinks | `the_hint_shows_then_dims_then_goes` |
| watch mode | The recovered opening: the pass scene opens out from the middle over the fail scene | Once per fix, over 2 seconds (3.5 under reduced motion, with no sparkles); each cell changes once as the edge passes | `the_opening_never_flashes_and_ends_on_the_new_scene` |
| every scene | A change of scene with `n` or `b` | At most once every 0.4 seconds | `SCENE_DEBOUNCE` in `src/app.rs` and `src/watch/mod.rs` |
| every scene | A change of scene on a timer, with `--shuffle`, `--every` or the config's interval | Every few minutes, 5 by default, never more often than every 3 seconds however small the time asked for | `the_timer_never_changes_scenes_faster_than_the_floor_and_always_moves` |
| watch mode with a command | The change between passing and failing | Once per finished run. Files are looked at every 0.7 seconds and a run starts only once they have been still for 0.3 seconds | `a_run_passes_fails_and_reruns_on_a_change` |
| follow mode | The change between passing, failing and a typo | Once per command, never more often than every 0.4 seconds. Commands that finish closer together than that show only the latest | `following_reads_only_new_commands_and_skips_interruptions` |
| Lua scenes | Anything the script draws | A frame that would make a fourth large change in a second is held back; a scene held back more than ten times in ten seconds is stopped | `a_flashing_script_is_damped_then_stopped` |
| sea | Nothing changes in place; the fish, bubbles, seaweed, boat, whale and the highlights on the water only move, the sun stays still | Half speed under reduced motion, with fewer bubbles and fewer, slower highlights | `calm_and_the_flash_limits_hold_in_every_theme`, `passes_the_shared_checks` |
| rain | A bolt of lightning, one thin jagged line with a short fork hanging from the cloud behind the skyline, never the whole screen | At least 5 seconds since the last (`MIN_LIGHTNING_GAP`), whatever resizes happen, on screen for 0.12 seconds (`FLASH_LENGTH`), never under reduced motion | `lightning_keeps_at_least_five_seconds_between_flashes`, `resizing_never_brings_lightning_early`, `lightning_never_strikes_under_reduced_motion`, `a_bolt_is_one_jagged_line_in_the_upper_part_and_never_lights_the_sky` |
| snow | Nothing changes in place; the flakes only move and the bank grows a third of a row a flake, half speed under reduced motion | Flakes fall at their own speed, half speed under reduced motion | `passes_the_shared_checks` |
| starfield | A star's brightness: a bright star through a dim point, a full point and a small cross, a faint star between a dimmed and a full point | One cycle takes at least 2.5 seconds (`MIN_TWINKLE_PERIOD`), at least 5 under reduced motion | `stars_twinkle_slowly_and_the_moon_hangs_top_right`, `a_bright_star_grows_into_a_cross_and_shrinks_back` |
| code-rain | A cell of a column's trail, as the column moves down or a character is swapped | At most three times a second under reduced motion (`CALM_MAX_SPEED`), where characters are never swapped | `calm_trails_change_a_cell_at_most_three_times_a_second`, `calm_columns_never_change_their_characters` |
| fireflies | A light's brightness, through a dim point, a lit point in a soft glow and a bright point; by day a mote of pollen's glow and a butterfly's wings, through open, half open and shut | One light cycle takes at least 2.5 seconds (`MIN_PULSE_PERIOD`), at least 10 under reduced motion; one wing beat takes 1.3 seconds (`FLUTTER_PERIOD`), 2.6 under reduced motion | `lights_pulse_slowly_and_glow_by_night`, `passes_the_shared_checks` |
| aurora | The curtains' thickness, which swells and shrinks by a quarter; the curtains otherwise only drift | One breath every 21 seconds (`BREATH_RATE`), every 42 under reduced motion | `passes_the_shared_checks`, `a_calm_aurora_moves_at_half_speed` |
| campfire | A row of the flame, which takes a new lean and width; rows change on staggered timers. The glow never changes and the embers only move | Each row every 0.6 seconds (`FLICKER_PERIOD`), so no flame cell more than twice a second; every 1.5 seconds under reduced motion (`CALM_FLICKER_PERIOD`), so a cell at most once a second | `no_flame_cell_changes_more_than_twice_a_second`, `each_row_takes_a_new_shape_once_a_period` |
| waves | A gull's wings, through four positions; the sun is still and the swell only moves | One beat takes 1.6 seconds (`FLAP_PERIOD`), at least 3.2 under reduced motion, never quicker on lively | `the_sea_fills_below_the_horizon_and_gulls_fly_above_it` |
| zacess | Nothing changes in place; the mark, the ribbons and the halo only move | Half speed under reduced motion | `passes_the_shared_checks` |
| forest-stream | The shafts of light drifting and the ripples running down the stream | One shaft's width every 40 seconds (`LIGHT_PERIOD`), 80 under reduced motion; the ripple pattern repeats about every 3.9 seconds (a flow rate of 1.6 radians a second), twice as slowly under reduced motion | `passes_the_pixel_checks` |
| desert-night | A star switching between its bright and dim shade and a camel's legs changing step | A star twice every 7 seconds (`STAR_PERIOD`) at most on every level, half as often under reduced motion; a camel's legs twice every 1.2 seconds (`STRIDE_PERIOD`), half as often under reduced motion; no stars in light themes | `the_moon_rises_then_holds_and_stars_change_slowly`, `passes_the_pixel_checks` |
| rooftops | One window lighting up or going dark and a flying pigeon's wings | One window every 2.4 seconds at most (`WINDOW_GAP`, plus up to 2 seconds more), every 4.8 under reduced motion and never faster on lively; wings twice every 0.8 seconds (`WING_PERIOD`), half as often under reduced motion | `windows_change_one_at_a_time_and_pigeons_land_on_ridges` |
| lily-pads | A frog hopping and the rings of ripples it leaves | One hop every 6 seconds at most at the standard pace (`HOP_GAP`), every 12 under reduced motion and no more often on lively; each ring spreads and fades over 2.4 seconds | `frogs_hop_from_pad_to_pad_slowly_and_never_share` |
| koi-pond | A ripple's ring as it widens and fades and a koi's tail as it sweeps | A ripple widens over 3.2 seconds (`RIPPLE_LIFE`), one every 4 to 8 seconds; a tail sweep takes 2.6 seconds (`TAIL_BEAT`); both take twice as long under reduced motion, ripples every 8 to 14 seconds | `tails_and_ripples_move_slowly_and_more_slowly_when_calm`, `passes_the_shared_checks` |
| harbour | A lamp's reflection and the boats' reflections on the water; a mast top leaning and the water lapping at each hull | The shimmer changes every 0.8 seconds (`SHIMMER_PERIOD`), every 1.6 under reduced motion; a mast and the lapping change about once a second over a 4 second sway (`SWAY_PERIOD`) and are still under reduced motion; the lamps' reflections show only at night | `the_shimmer_and_the_sway_stay_slow`, `passes_the_shared_checks` |
| lanterns | Nothing changes in place; lanterns only rise and sway a column either way and the floating lanterns and swell only drift | One sway takes 7 seconds (`SWAY_PERIOD`), 14 under reduced motion; rising and drifting at half speed under reduced motion | `lanterns_sway_slowly_and_rise`, `passes_the_shared_checks` |
| balloons | Nothing changes in place; balloons only rise and drift and clouds only drift | Balloons rise at most 1 pixel a second at the standard level, half speed under reduced motion | `balloons_rise_and_the_hills_stay_put`, `passes_the_shared_checks` |
| countryside | A puff of smoke, through three sizes | One size a second (`PUFF_STAGE`), a new puff every 0.8 seconds; one size every two seconds and a puff every 1.6 seconds under reduced motion | `the_land_slides_past_but_the_train_stays_in_view`, `passes_the_shared_checks` |
| winter-cabin | A puff of smoke, through three sizes | One size every 1.2 seconds (`PUFF_STAGE`), a new puff every 1.1 seconds; every 2.4 seconds and a puff every 2 seconds under reduced motion | `the_cabin_window_is_lit_and_smoke_rises`, `passes_the_shared_checks` |
| autumn | A falling leaf's turn, through four positions | One turn every 0.6 seconds (`LEAF_TURN`); no turning under reduced motion | `leaves_turn_slowly_and_the_pile_stays_low`, `passes_the_shared_checks` |
| blossom | A falling petal turning over and the bird facing the other way | A petal every 0.7 seconds (`PETAL_TURN`) and the bird every 6 seconds (`BIRD_LOOK`); no turning petals and the bird every 12 seconds under reduced motion | `petals_turn_slowly_and_the_bird_looks_rarely`, `passes_the_shared_checks` |
| city-night | One window lighting up or going dark | One window every 1.2 seconds (`WINDOW_PERIOD`), every 2.4 under reduced motion | `one_window_changes_at_a_time_and_never_quickly`, `passes_the_shared_checks` |
| orbit | Nothing changes in place; the moon and the storm only move | The moon goes round once every 40 seconds (`ORBIT_PERIOD`) and the storm crosses once every 60 (`SPOT_PERIOD`), both at half speed under reduced motion | `the_moon_orbits_slowly_and_passes_behind_the_planet`, `passes_the_shared_checks` |
| constellations | A figure's stars turning bright yellow, its lines changing colour as they are held and fade and its name appearing and changing colour | The lines are drawn at 9 pixels a second, held for 6 seconds, then change every 1.2 seconds (`FADE_STEP`) as they fade; twice as long under reduced motion | `figures_are_drawn_named_and_faded_one_at_a_time`, `passes_the_shared_checks` |
| tea | A wisp of steam swaying a pixel or so either way as it rises | A sway takes 2.4 seconds (`SWAY_PERIOD`), so a wisp crosses a column at most twice in that time; every 4.8 seconds under reduced motion, with half as many puffs | `steam_rises_from_the_mug_and_sways_slowly`, `passes_the_shared_checks` |
| cat-nap | The cat's back as it breathes, its tail tip and a z changing size and fading | The back twice every 4 seconds (`BREATH_PERIOD`), the tail twice every 7 (`TAIL_PERIOD`), a new z every 3 seconds; all half as often under reduced motion | `the_cat_breathes_slowly_and_its_tail_stirs_rarely`, `passes_the_shared_checks` |
| garden | A stem growing a pixel, a bud opening and a butterfly's wings | A stem pixel at most once a second, wings open and close once every 1.2 seconds (`FLUTTER_PERIOD`), both half as often under reduced motion | `stems_grow_slowly_and_wings_beat_slowly` |
| tiles | One tile taking a new colour, filling or settling to its outline | One tile every 3 seconds on a small grid (`CHANGE_PERIOD`) and never more often than every 0.7 seconds on a large one (`MIN_CHANGE_GAP`), half as often under reduced motion | `one_tile_changes_at_a_time_and_never_quickly` |
| ribbons | Nothing changes in place; the ribbons only flow and twist | No crest or twist faster than 4 columns a second (`MAX_DRIFT`), half speed under reduced motion | `ribbons_drift_and_twist_no_faster_than_the_limit` |
| sunflower-field | Nothing changes in place; the heads sway and the bees and clouds only move | One sway about every ten seconds (`SWAY_RATE`), at least eight, slower and smaller under reduced motion | `the_sway_is_slow_and_the_roots_stay_put`, `passes_the_pixel_checks_in_every_theme` |
| mountain-lake | The heron changing pose and a ring spreading from its beak | The heron holds each pose at least 3 seconds (`HERON_HOLD`), in practice 8 to 14 seconds upright and 4 to 7 fishing, twice as long under reduced motion; one ring per fishing pose, spreading at `RING_GROWTH` and fading over 5 seconds | `the_lake_mirrors_the_land_and_the_heron_holds_each_pose`, `passes_the_pixel_checks_in_every_theme` |
| tide-pools | The shimmer on the pools and the sea, an anemone's tentacles and a crab's legs | The shimmer moves at 0.9 radians a second (`SHIMMER_RATE`), slower and sparser under reduced motion; one wave about every five seconds (`WAVE_RATE`); a leg position holds 0.4 seconds (`CRAB_STEP`); all half as often under reduced motion | `crabs_keep_out_of_the_pools_and_legs_step_slowly`, `passes_the_pixel_checks_in_every_theme` |
| paper-boats | Nothing changes in place; the boats, ripples, reeds and clouds only move | Boats bob once every six seconds or so (`BOB_RATE`), the nearest drift at `FLOW`, the reeds sway at `REED_RATE`; half speed under reduced motion | `boats_drift_downstream_and_stay_on_the_water`, `passes_the_pixel_checks_in_every_theme` |
| windmills | Nothing changes in place; the sails turn and the clouds only move | One turn every sixteen seconds or so (`SAIL_TURN`), no sail tip faster than 6 pixels a second (`MAX_TIP_SPEED`), half speed under reduced motion | `the_sails_turn_slowly_and_the_mills_stand_in_a_row`, `passes_the_pixel_checks_in_every_theme` |
| picnic | Nothing changes in place; the kites, their tails and the clouds only move | Kites drift at `KITE_DRIFT`, one tail ripple every four seconds or so (`TAIL_RATE`), half speed under reduced motion | `kites_stay_in_the_sky_on_their_strings`, `passes_the_pixel_checks_in_every_theme` |
| drone-landing | The beam's sweep, the hovering drone and the readout changing once from MAPPING TERRAIN to POSITION FIX GOOD | The sweep crosses the beam and back once every 3.2 seconds (`SWEEP_PERIOD`) and the hover bobs a pixel once every 4 seconds (`BOB_PERIOD`), both half as often under reduced motion; the readout changes once a landing, at 16 seconds (`APPROACH`) | `the_rotors_spin_down_slowly_and_the_sweep_is_slow` |
| kettle-boil | A curl of steam, the lid lifting, bubbles climbing the gauge, the lamp switching off and the mug filling | A curl sways twice every 2.6 seconds (`SWAY_PERIOD`) and the lid lifts once every 0.9 seconds (`RATTLE_PERIOD`) for the last seconds of the boil, both half as often under reduced motion; the lamp, the gauge level and the tea each change once a boil | `steam_sways_and_the_lid_rattles_slowly` |
| rocket-launch | The countdown on the board, the engine's glow swelling and the flame's waver | One number a second at most on every level (`COUNT`), every two seconds under reduced motion; the glow swells over four seconds (`IGNITE` to `LIFT`) and the flame wavers once every three (`FLAME_PERIOD`), both half as often under reduced motion | `the_countdown_and_the_glow_change_slowly_on_every_level` |
| pit-stop | The board's stop time and the lollipop swinging from red to green and back | The board changes once a stop, every 30 seconds (`CYCLE`); the lollipop swings up over a second (`LIFT`) and back down once the car is clear; half as often under reduced motion | `the_car_stops_in_its_box_then_leaves_with_new_wheels_and_a_time` |
| pcb-bring-up | An LED warming up and lighting, the legend lighting, the scope's trace turning to a square wave and the supply's current reading | One LED every 1.4 seconds (`STEP`), each changing twice in 0.5 seconds (`WARM`), half as often under reduced motion; the legend and the trace change once a cycle; all fade together over 1.2 seconds about once a minute (`HOLD` plus the chain) | `leds_light_in_order_no_faster_than_the_step_and_end_on_power_good` |
| storm | A bolt of lightning, a thin forked line from the cloud with a faint glow where it leaves it, never a large area; the tree's crown as it bends in a gust | A bolt once every 5 seconds since the last (`MIN_LIGHTNING_GAP`), whatever resizes happen, the first one 5 seconds after the storm appears, on screen for 0.12 seconds (`FLASH_LENGTH`), never under reduced motion; one gust every 4.5 seconds (`GUST_PERIOD`), every 9 under reduced motion | `the_storm_keeps_the_lightning_limits`, `resizing_never_brings_lightning_early`, `the_bolt_is_a_thin_line_and_the_tree_bends_slowly` |
| flood | The water's surface row as it swells and each piece of debris as it bobs a pixel | The surface at most twice every 12 seconds (`SWELL_PERIOD`), under half a row each way (`SWELL_PIXELS`); the rise takes 45 seconds (`RISE_SECONDS`); debris bobs down and back once every 4.8 seconds; both half as often under reduced motion | `the_surface_moves_at_most_twice_a_swell`, `the_water_rises_slowly_then_holds_below_the_roofs` |
| listing-ship | The ship's angle as it lists and rolls | Under two degrees a second, so the bow moves under a row a second; the list takes 50 seconds (`LIST_SECONDS`) and a roll 11 seconds (`ROLL_PERIOD`), both twice as long under reduced motion | `the_ship_lists_slowly_then_rolls_gently_at_its_angle` |
| corrosion | A pixel of the wall stepping from steel to film, rim, scale and core as a patch's edge creeps past it | The edge moves well under half a pixel a second even on lively, each patch growing over 150 seconds of the scene's clock (`GROW_SECONDS`); a drop slides at 1.6 pixels a second (`DROP_SPEED`) and rests 9 seconds (`DROP_REST`); all at half speed under reduced motion | `the_edge_of_a_patch_creeps_well_under_a_pixel_a_second`, `rust_creeps_slowly_then_holds_and_never_takes_the_whole_wall` |
| tangle | The cables as they tangle and sway | The tangle starts 45 percent formed (`START`) and grows to full over 45 seconds (`TANGLE_SECONDS`), at most about a fortieth of the whole a second on lively; the sway turns at 0.35 radians a second (`SWAY_RATE`), a fraction of a pixel a second; both at half speed under reduced motion | `the_tangle_grows_slowly` |
| wilt | The flower heads losing petals and fading, the stems and leaves drying, the soil cracking | Over the 50 second wilt (`WILT_SECONDS`) a flower head changes four times, a stem pixel steps through five tones and each crack opens once; the drooped stems sway once every 9 seconds (`SWAY_PERIOD`); all half as often under reduced motion | `stems_droop_leaves_fall_and_land_on_the_sill` |
| fog | The fog thickening over a cell and a lamp dimming as the bank takes it | The front rolls in over 50 seconds (`ROLL_SECONDS`) and the hidden share changes by under a hundredth of the screen a frame; the billows drift 1.2 columns a second (`DRIFT`) over billows 14 columns wide (`PATCH`); half speed under reduced motion | `the_fog_rolls_in_slowly_and_swallows_the_lamps` |
| broken-bridge | A splash where a fragment lands, a small burst then two drops either side | One fragment every 4 seconds (`FRAGMENT_PERIOD`), the splash changing twice in 0.7 seconds (`SPLASH_LENGTH`); half as often under reduced motion, with the splash taking twice as long | `fragments_fall_slowly_and_splash_once_each` |
| leak | The bead swelling under the crack, a ring spreading on the pail's water and the water level creeping up | One drip every 1.2 seconds (`DRIP_PERIOD`), the bead taking two steps as it swells; each ring spreads over 1.4 seconds (`RING_LIFE`), at most three at once; the pail fills over 40 drips (`DRIPS_TO_FILL`) and the puddle widens over 45 more (`DRIPS_TO_SPREAD`); all half as often under reduced motion | `drips_are_slow_and_never_quicker_on_lively`, `the_pail_fills_then_the_puddle_spreads_and_holds` |
| blackout | A window going dark, then perhaps a candle | One window at a time over 90 seconds (`DARKEN_SECONDS`), at least 0.25 seconds apart (`MIN_WINDOW_GAP`), half as often under reduced motion; each window changes at most twice, the candle five seconds after it goes dark (`CANDLE_DELAY`) | `no_window_changes_more_than_once_a_second_while_the_lights_go_out`, `windows_go_dark_one_at_a_time_and_stay_dark` |
| traffic-jam | An exhaust puff swelling and thinning as it drifts | One puff a car every 3 seconds (`PUFF_PERIOD`), each fading out over 2.8 seconds (`PUFF_LIFE`); half as often under reduced motion; the signal and the brake lights never change | `puffs_are_slow_on_every_level_and_sprites_are_even` |
| erosion | A piece of the face breaking away and a splash | One piece every 2.2 seconds (`CRUMBLE_PERIOD`), then one pebble every 7 seconds (`PEBBLE_PERIOD`); the splash changes twice in 0.7 seconds (`SPLASH_LENGTH`); half as often under reduced motion | `pieces_break_away_slowly_on_every_level` |
| drought | A crack opening along a plate edge and the puddle shrinking a step | The cracks still to open after `FIRST_CRACKS` open at the pace of a full spread over 60 seconds (`CRACK_SECONDS`), each edge opening once; the puddle loses one step every 6 seconds (`PUDDLE_SHRINK`); both half as often under reduced motion | `the_puddle_shrinks_one_step_at_a_time_at_most_every_few_seconds`, `paints_in_pixels_in_every_theme` |
| volcano | Nothing changes in place; the lava fronts, the glints on the lava, the ash and the embers only move | The ash rises 1.6 pixels a second (`ASH_RISE`) and the glints run 1.4 pixels a second (`RUN_SPEED`), under 2.5 at every level and half speed under reduced motion; each front creeps under a pixel row a second (`LAVA_SECONDS`) | `the_ash_rises_slowly`, `lava_creeps_down_slowly_then_pools_and_holds`, `passes_the_pixel_checks` |
| rockslide | A puff of dust swelling and fading where a boulder strikes | One boulder every 2.4 seconds at the standard pace (`ROCK_PERIOD`), then one pebble every 6 seconds (`PEBBLE_PERIOD`); each puff fades over 1.8 seconds, at most eight at once; under reduced motion a boulder every 9.6 seconds, at most three puffs and the puffs fade at half the rate | `boulders_tumble_down_and_block_the_road_then_the_slide_holds`, `passes_the_pixel_checks` |
| dust-storm | The dust thickening over the scene and the billows on the wall swelling and shrinking | The wall takes 60 seconds to cross (`STORM_SECONDS`) and no second thickens the screen by a twentieth or more; each billow swells and shrinks by under a tenth of its size once every 21 seconds or so; half speed under reduced motion | `the_wall_swallows_the_far_poles_first_and_then_holds`, `calm_storms_roll_in_at_half_the_rate` |
| thin-ice | A crack growing a pixel at a time and the hole opening | The cracks spread over 60 seconds (`CRACK_SECONDS`), each pixel appearing once and never more than 24 in a frame; the hole opens over 40 seconds (`HOLE_SECONDS`); both at half the rate under reduced motion; the snow only moves | `cracks_creep_out_slowly_then_hold_and_survive_a_resize`, `calm_cracks_spread_at_half_the_rate` |
| short-circuit | A burst of sparks at the break and the glow round it swelling | One burst every 1.6 to 2.6 seconds (`SPARK_PERIOD` plus up to a second), never sooner than 1.44 seconds after the last, each spark a pixel living 0.9 seconds; the glow swells once every 5 seconds (`GLOW_PERIOD`); both half as often under reduced motion | `sparks_keep_their_period_on_every_level`, `passes_the_pixel_checks` |
| overgrown | The tips of the vines and weeds stirring and the growth appearing a pixel at a time | The tips stir a pixel either way once every 9 seconds (`SWAY_PERIOD`), never twice within a second; the growth spreads over 90 seconds (`GROW_SECONDS`), at most 20 pixels a frame; both half as often under reduced motion | `the_tips_stir_slowly`, `the_growth_creeps_on_then_holds_and_survives_a_resize` |
| landing-abort | The beam's sweep, the hovering drone, the lamp turning amber and the readout changing | As `drone-landing`: the sweep crosses and back once every 3.2 seconds (`SWEEP_PERIOD`) while the beam maps and the hover bobs a pixel once every 4 seconds (`BOB_PERIOD`), both half as often under reduced motion; the lamp and the readout change once, 16 seconds in (`APPROACH`), 32 under reduced motion | `the_rotors_spin_down_slowly_and_the_sweep_is_slow`, `passes_the_shared_checks` |
| kettle-cold | The lamp coming on and going out, a bubble climbing the gauge and the note appearing | Once each: the lamp comes on 0.6 seconds in and goes out at 4 seconds (`COLD_CLICK`), the note appears 1.5 seconds later; one bubble climbs the gauge a row at a time while the lamp is on; all twice as late or half as fast under reduced motion | `the_kettle_clicks_off_cold_and_a_note_says_to_try_again`, `passes_the_shared_checks` |
| launch-scrub | The board turning to HOLD, the steam venting and the mist swelling | The board changes once (`HOLD_AT`); one puff from each vent every 0.8 seconds (`VENT_PERIOD`), moving under half a pixel a frame; the mist swells once every 9 seconds; half as often under reduced motion | `the_board_holds_the_arm_swings_home_and_the_rocket_never_lifts`, `steam_drifts_slowly_from_the_rocket` |
| pit-stop-stuck | The time on the board and the stuck hub turning to the error colour | The board changes one second at a time, never more than once a second at any level and half as often under reduced motion, holding at 9:59 (`BOARD_MAX`); the hub changes once, 1.5 seconds after the guns go on (`STUCK_AFTER`) | `the_board_climbs_no_faster_than_once_a_second_and_the_wheel_stays`, `paints_in_pixels_in_every_theme` |
| dead-led | The probe moving to the dark LED and the beam's spot on the scope | The probe moves over 7 seconds (`PROBE_MOVE`), 14 under reduced motion, starting a second in; the spot moves along the flat trace at 9 pixels a second (`SWEEP`) and holds still under reduced motion | `the_dead_led_never_lights_and_the_probe_reaches_it`, `the_scope_trace_stays_flat_and_the_legend_stays_dim` |

Things that move, such as drops, fish, flakes and fireflies, change the pixels they pass through
as they go. These are small shapes moving across the screen, not flashes. Under reduced motion
every one of them moves at half speed or slower.

Every built-in scene is painted in solid pixels and also goes through `check_pixel_scene` in
`src/scenes/testing.rs`. In every theme in the palette it runs the flash meter for 30 seconds at
every motion setting and resizes the scene from one cell to 400 by 120 and back. It checks that a
paused scene draws the same frame again, that most of the picture is background imagery below the
3 to 1 non-text floor, that shapes keep an outline when colour is off, that no colour drifts far in
the 256 colour table and that no shape lands on the same 256 colour entry as the backdrop behind
it. It also checks that the watch mode's banner and caption keep the floor over the scene and that
reduced motion changes fewer than three quarters as many cells as the standard pace over 20
seconds.

## Colour and contrast

- Every colour starts from the Zacess palette, version 1.0.0, in all ten themes: the theme's
  text, muted and accent colours, its syntax colours and its sixteen terminal colours or a mix of
  them. A few daylight colours in the built-in scenes are mixed partly towards a fixed hue (fire
  yellow, white snow and cloud, paper or a planet's bands) so they read as what they are; they are
  held to the same floors as every other colour.
  ANSI black is never used because in Zacess Deep Contrast Dark it is the background itself.
- Each theme's floor is the text contrast the palette sets for it: 7 to 1, the WCAG AAA level, on
  the first eight themes and 4.5 to 1, the AA level, on Warm Dark and Warm Light. Text and every
  colour role a scene draws with keep that floor on the theme's background; pixel shapes are held
  to the non-text floor described below. A terminal colour below
  the floor is replaced by its bright or plain twin when that keeps the floor. Otherwise it keeps
  its own hue and only its OKLab lightness moves away from the background until it reaches the
  floor (`fit_ansi` in `src/theme.rs`), so on Cream the yellow becomes a deep ochre instead of
  turning into the muted ink. Only a colour no lightness can lift is swapped for the palette
  colour nearest in hue.
- Scenes are painted in solid pixels: each character cell is an upper half block whose foreground
  is the top pixel and whose background is the bottom pixel. Background imagery, such as a sky, a
  field or the sea, is a tint of the theme's background mixed in the OKLab space and held below
  the 3 to 1 non-text floor (`IMAGERY_CEILING` in `src/pixels.rs`), so it reads as background and
  never competes with text. Pixel shapes that carry meaning keep at least that 3 to 1 against
  the background, solid fail scene shapes a little over it; they are not held to the text floor.
  The ratio is computed by `Rgb::contrast` in `src/colour.rs` and three tests hold the floors:
  `every_drawable_colour_keeps_the_contrast_floor_in_every_theme` checks every colour a theme
  offers; `every_colour_role_keeps_the_theme_floor` checks every role a scene draws with; each
  scene's `passes_the_shared_checks` draws the scene in all ten themes and checks the text colour
  of every frame against the text floor over a simulated minute at 80 by 24, long enough to
  include lightning, shooting stars and the whale, plus sampled frames at 20 by 8 and 120 by 40.
- The light shade `░` covers about a quarter of its cell, so it reads fainter than its measured
  colour. No built-in scene draws it now that they are painted in pixels; `ThemeData::faint_shade`
  still gives `▒` instead on a theme with a 7 to 1 floor for any scene that does.
- Where two colours touch inside one shape, the Zacess mark's band and its page or tile, a line of
  background pixels keeps them apart, so the band never depends on the contrast between the two
  colours. `the_band_never_touches_a_pixel_of_another_shape` checks it in every theme.
- The watch mode's banner, list and status bar use the theme's green for passing and its error
  and warning colours for problems. Where the error or warning colour is below the theme's floor
  its red or yellow stands in. `every_overlay_colour_keeps_the_floor_in_every_theme` draws every
  state in all ten themes and checks every colour. It also checks that the error colour differs
  from both the warning and the passing colours.
- Lua scenes cannot name a colour. They draw with a palette role such as `text`, `accent`,
  `error` or `syntax.keyword`, which the program turns into the theme's colour, falling back to
  the text colour where a role is below the floor. `every_role_keeps_the_floor_in_every_theme`
  holds it.
- Colour is never the only signal. Fish of different kinds are different shapes, a board that says
  what is wrong (BRIDGE OUT, KEEP BACK, ROAD CLOSED) carries its words and every shape that carries
  meaning stands out from the imagery by its contrast as well as its colour. In the watch mode
  passing reads `✓ passing` with ALL GOOD, failing reads `✗ failing` with BUILD FAILED, errors are
  marked with a cross and warnings with `!` as well as their own words. With `NO_COLOR` set, every
  character is drawn in the terminal's own text colour and every scene stays readable: a pixel at or
  above the 3 to 1 ceiling becomes a solid half block (`▀`, `▄` or `█`) and the background imagery
  becomes blank, so each shape keeps its outline (`plain_glyph` in `src/pixels.rs`, held by
  `without_colour_only_shapes_keep_an_outline` and by `check_pixel_scene` in every scene).
- The built-in scenes paint their own background in the theme's colours. Text that sits on the
  plain background, such as the help line, assumes the terminal is already on the same Zacess
  theme, as do Lua scenes, which set no background colour. On a terminal with a different
  background the measured ratios for those do not apply.
- Many scenes have a look of their own for the light themes, such as a day instead of a night,
  chosen from the theme's `dark` flag. The light look is held to the same checks. On a light theme
  the snow is white imagery lighter than the sky, with the dark pines carrying the outline. The
  Zacess mark is drawn as its tile in the text colour with the page left as the background, so it
  keeps the look of the real mark.

## Sound

A scene on its own is silent unless the background sound is turned on. The watch and follow modes
can play nine short events in one of eight sound sets: `bells`, `marimba`, `kalimba`, `pad`,
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
| clean | A passing run has no warnings after the last one had some, at 70 percent of the level | Yes |
| stopped | The watch stopped on its own, at 80 percent of the level | Yes |

- Sound is never the only signal. Every event has the same state on screen at the same moment: the
  banner, the scene and the status bar show passing, failing or recovered. The status bar shows
  `running` while a rebuild runs and the warning count.
- Every set plays the same phrase for each event, a rising triad for a pass, a falling fourth for a
  failure, the triad landing an octave up for a fix, a rising minor third for warnings, a rising
  fifth when they clear, two soft low notes a step apart for a typo and a slow falling triad when
  the watch stops, so the meaning does not change with the timbre. A typo shows TYPO on screen as
  well as its tone, the warning count shows 0 when they clear and a stopped watch says why on
  screen.
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
- A quiet loop that suits the scene: water, rain, wind, fire, a night meadow, a stream, birdsong,
  a machine hum, a distant city or a slow pad. One of them can play under every scene with
  `--ambient-sound`, `ZACESS_AMBIENT_SOUND` or `sound` under `[ambient]`. It carries no meaning and
  nothing depends on hearing it.
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
> The built-in scenes are painted with the upper half block `▀` on a coloured background. With
> colour off they use `▀`, `▄` and `█`. A terminal font without them shows boxes or gaps in place
> of the picture. Text over a scene is plain ASCII.

- Colours are sent as exact 24 bit colour only when the terminal says it supports it. Elsewhere,
  including Terminal.app before macOS 26, the Linux console, tmux or screen without true colour and
  the classic Windows console, each colour is sent as the nearest entry in the 256 colour table
  that still keeps the floor. A pixel keeps no floor, so it is sent as its plain nearest entry.
  A cell whose two pixels land on one entry is sent as a space of that colour. The pixel
  checks hold every colour within 0.16 in OKLab of its entry and keep shapes off the backdrop's
  entry, so a picture keeps its shapes in 256 colours. `--colour truecolor` or `--colour 256` overrides the guess. The
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
