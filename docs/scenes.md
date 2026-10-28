# The scenes

What each scene draws, how it fits the screen, which of the theme's colours it uses and what
changes under reduced motion. Every scene implements the `Scene` trait in `src/scene.rs` and lives
in its own file under `src/scenes`.

Common to all of them:

- The scene is laid out again on every resize, from 20 columns by 8 rows upwards. Counts of fish,
  drops, flakes, stars and so on scale with the area within a floor and a ceiling.
- Every colour is a role from the theme (its text, muted and accent colours, its syntax colours or
  its terminal colours), never a fixed value, so every scene passes the contrast test in all eight
  themes.
- Reduced motion halves every speed and caps the frame rate at 12 a second. The scene-specific
  changes are listed below.
- The same seed, size and frame count always produce the same frame.

## sea

A wave line near the top, fish of three sizes (`><>`, `><(o>` and `><(((o>`, facing the way they
swim) in five colours, bubbles that rise from fish and from the seabed, seaweed that sways on the
sandy bottom and a boat drifting along the surface. On a sea at least 30 columns wide with at least
six rows of water, a whale crosses the deep water now and then, its first appearance between 8 and
20 seconds in and then every 20 to 50 seconds.

- Colours: the surface in cyan, sand in yellow, seaweed in green, bubbles in the muted colour, fish
  in yellow, magenta, bright cyan, red and bright blue, the whale in blue and the boat in the text
  colour.
- Small screens: the surface moves up a row and the boat loses its mast. The whale is left out.
- Reduced motion: fish swim at half speed and fewer bubbles rise.
- Tests: the boat, the surface, the seaweed and a fish are in the frame; a whale crosses an 80 by
  24 sea within a minute and never a 20 by 8 one; fish stay in the water.

## rain

Clouds along the top row, drops at two depths (near ones as `|` falling fast, far ones as `:`
falling slower, with a slight wind), a puddle line along the bottom and a ripple where each near
drop lands, growing from `o` to `(   )` over 0.6 seconds. Every 5 to 15 seconds a bolt of lightning
is drawn for 0.12 seconds as one jagged line in the upper third of the screen. It is a shape in the
corner of the eye, never a whole-screen flash. The gap between bolts is never under five seconds
whatever the random numbers say (`MIN_LIGHTNING_GAP`).

- Colours: near drops in bright blue, far drops in blue, ripples in cyan, the puddle line in the
  muted colour, clouds in grey and the bolt in bright yellow.
- Small screens: a screen under 7 columns or 6 rows gets no bolt.
- Reduced motion: no lightning at all and rain at half speed.
- Tests: drops and ripples are drawn, no bolt ever appears under reduced motion, the gaps between
  bolts are at least five seconds over five simulated minutes and a tiny screen never gets one.

## snow

Flakes at two depths (`*` near, `.` far) drift down with a slow sideways sway and settle where
they land. Three flakes raise a column's bank by one row and a column that stands more than a row
above its neighbour sheds into it, so the bank stays rounded. The bank is drawn in full blocks with
a half block on top for a half row and never rises above a third of the screen.

- Colours: near flakes in the text colour, far flakes in the muted colour, the bank in the text
  colour on a dark theme and in grey on a light theme, so it does not become a slab of black.
- Reduced motion: flakes fall at half speed.
- Tests: flakes fall and a bank forms within forty seconds; the bank never passes a third of the
  screen over ten simulated minutes.

## starfield

Stars that brighten and fade through `.`, `+` and `*` (bright stars) or `.` and `:` (faint ones),
each on its own cycle of 2.5 to 6 seconds, with a few tinted yellow or cyan. A crescent moon hangs
top right when the screen is at least 24 by 8. Every 6 to 16 seconds a shooting star crosses the
upper sky at 14 cells a second with a short grey tail, then is gone.

- Colours: bright stars in the text colour, bright yellow and bright cyan, faint stars in the muted
  colour, the moon in bright yellow, the shooting star's head in the strongest text colour and its
  tail in grey.
- Reduced motion: cycles take 5 to 10 seconds, shooting stars come every 15 to 40 seconds and move
  at half speed.
- Tests: no star has a cycle shorter than `MIN_TWINKLE_PERIOD`; the moon is drawn at 60 by 20 and
  not at 20 by 8; a shooting star appears within forty seconds and the calm one is slower.

## code-rain

Every column of the screen is a stream of characters (letters, digits and symbols, all ASCII) that
falls from the top at its own speed, with a bright head and a tail that fades through the column's
colour into grey. Characters in a falling column change now and then. A column that has fallen off
the bottom waits a while before falling again; the wait and the speed both scale with the height
of the screen so a short terminal stays full and a tall one is not a blur.

- Colours: each column takes one of the theme's nine syntax colours, the head is the strongest
  text colour and the last two characters of the tail are grey.
- Reduced motion: columns fall at half speed and the characters in a column never change.
- Tests: columns fall in syntax colours with a bright head; under reduced motion a column's
  characters stay the same while it falls.

## fireflies

A dark meadow: a row of grass tufts along the bottom with a few taller stalks above it, plus
lights that wander slowly with a gentle random walk, bouncing off the edges and staying above the
grass. Each light pulses on its own cycle of 2.5 to 5 seconds, from not drawn through `.` and `o`
to `*`.

- Colours: grass in green, lights in yellow and, at their peak, bright yellow.
- Reduced motion: cycles take 5 to 9 seconds and the lights wander at under half the speed.
- Tests: no light has a cycle shorter than `MIN_PULSE_PERIOD`; the lights reach every brightness;
  they stay inside the screen and above the grass.

## aurora

Two or three bands of colour across the upper half of the sky, each a slow wave made of two sine
waves whose phase drifts at a different speed per band, drawn in three shades (`▓`, `▒`, `░`) from
the band's centre outwards. Their thickness breathes gently over about twenty seconds. Faint stars
sit behind the bands and a mountain ridge, drawn as a single outline of `/`, `\` and `^`, runs
along the bottom.

- Colours: the bands in green, cyan, magenta and blue, stars in the muted colour, the ridge in
  grey.
- Small screens: two bands rather than three under 10 rows.
- Reduced motion: the waves drift at half speed.
- Tests: the shaded cells are in the upper half and none in the lower, the picture changes over
  time, the ridge is drawn and the calm bands move at exactly half the speed.

## campfire

Logs drawn as `\=====/` on the ground with a steady glow of dots either side, a flame above them
built row by row from a base about a quarter of the screen wide, narrowing towards the top, with
three layers (`▓` core, `▒` middle, `░` edge) and `^` tips. The flame's shape is rolled every 0.34
seconds (`FLICKER_PERIOD`) and the drawn shape blends smoothly from the previous one to the next, so
the fire moves without ever jumping. Embers rise from the top of the flame, drift and fade from `*`
to `.` over one and a half to three seconds.

- Colours: core in bright yellow, middle and logs and glow in yellow, edges, tips and the last
  stage of an ember in red.
- Reduced motion: a new shape every second (`CALM_FLICKER_PERIOD`) and fewer, slower embers.
- Tests: the logs, the core and the tips are drawn and embers appear; over ten simulated seconds the
  shape is rolled no more than 30 times (and 25 or more, so the test cannot pass by doing nothing)
  and no more than 10 times under reduced motion.

## waves

A horizon at two fifths of the way down with a sun top right (on a screen at least 16 wide with at
least three rows of sky) and gulls crossing the sky, their wings moving through four positions over
a 1.6 second beat. Below the horizon every row is a wave of `~` crests and `-` troughs drifting
sideways, with longer and faster waves nearer the bottom and the occasional `'` of foam on the near
rows. The sun is drawn last, so a gull passes behind it.

- Colours: the horizon and far crests in cyan, near crests in bright cyan, troughs in blue, foam
  and gulls in the text colour, the sun in bright yellow.
- Reduced motion: the swell and the wing beats at half speed.
- Tests: the horizon row is unbroken, every row below it has waves, the sun and a gull are drawn at
  60 by 20 and the sun is drawn at 20 by 8 but not at 12 by 5. A wing beat changes under three times
  a second.

## zacess

The Zacess mark: a page with a folded top right corner and a band across its middle that reaches
past the page on both sides, drifting slowly across the screen and bouncing off the edges. The
page is picked from six sizes, the largest that fits with room to move, from 15 by 11 down to 3 by
3; a screen too small for any of them shows a still `Z`. When there is room the word `Zacess` sits
beneath the mark.

- Dark themes: the terminal background stands in for the mark's dark tile, the page is drawn in
  the text colour with `◣` cells for the fold in the muted colour and the band in bright yellow.
- Light themes: the tile is drawn in the text colour with its corners left off, the page is left as
  the background with `◥` cells for the fold and the band is in the theme's yellow.
- Reduced motion: it drifts at half speed.
- Tests: the page sizes chosen for 20 by 8 and 120 by 40, the page, band and fold drawn in both
  appearances and the mark moving while staying inside a 20 by 8 screen for a simulated minute.
