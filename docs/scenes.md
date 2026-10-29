# The scenes

There are 39 built-in scenes: the original ten, which also show while a build passes, 16 pass
scenes made for a passing build and 13 fail scenes made for a failing one, the storm among them.
`zacess-scenes list` marks each one pass, fail or general. Each section starts with a still from the
gallery, drawn by `zacess-scenes snapshot` in Zacess Deep Contrast Dark at 80 by 24, six seconds in.

What each scene draws, how it fits the screen, which of the theme's colours it uses and what
changes under reduced motion. Every scene implements the `Scene` trait in `src/scene.rs` and lives
in its own file under `src/scenes`.

Common to all of them:

- The scene is laid out again once a resize settles (no new size for 0.15 seconds), from 20
  columns by 8 rows upwards, so dragging a window edge never makes the screen blink between
  layouts. Counts of fish,
  drops, flakes, stars and so on scale with the area within a floor and a ceiling.
- Every colour is a role from the theme (its text, muted and accent colours, its syntax colours or
  its terminal colours), never a fixed value, so every scene passes the contrast test in all eight
  themes. On the High Contrast and Deep Contrast themes the floor is 7 to 1 and a terminal colour
  below it is swapped for the nearest palette colour that keeps it (on the light ones the bright
  yellow, for example, becomes the theme's darker brown or orange).
- Reduced motion halves every speed, including every sway, wobble and drift. It also caps the frame
  rate at 12 a second. The scene-specific
  changes are listed below.
- The motion level scales movement: calm at 0.7 times the standard speed with fewer things on
  screen, lively at 1.35 times with more. Anything that changes in place keeps the standard pace or
  slower at every level.
- Every scene passes `check_scene`: every colour in all eight themes at 20 by 8, 80 by 24, 120 by
  40 and 200 by 60 at all six motion settings, only characters every supported font has, odd sizes
  from one cell up without a panic and the flash meter over a simulated minute, which allows no
  more than three large changes of brightness in any second.
- The same seed, size and frame count always produce the same frame.

## sea

![The sea scene: Fish of several sizes, bubbles, seaweed, a boat on the surface and an occasional whale](images/scenes/sea.svg)

A wave line near the top, fish of three sizes (`><>`, `><(o>` and `><(((o>`, facing the way they
swim) in five colours, bubbles that rise from fish and from the seabed, seaweed that sways on the
sandy bottom and a boat drifting along the surface. On a sea at least 30 columns wide with at least
six rows of water, a whale crosses the deep water now and then, its first appearance between 8 and
20 seconds in and then every 20 to 50 seconds.

- Colours: the surface in cyan, sand in yellow, seaweed in green, bubbles in the muted colour, fish
  in yellow, magenta, bright cyan, red and bright blue, the whale in blue and the boat in the text
  colour.
- Small screens: the surface moves up a row and the boat loses its mast. The whale is left out.
- Reduced motion: fish swim, sway and wobble at half speed and fewer bubbles rise.
- Tests: the boat, the surface, the seaweed and a fish are in the frame; a whale crosses an 80 by
  24 sea within a minute and never a 20 by 8 one; fish stay in the water.

## rain

![The rain scene: Drops and puddle ripples, with distant lightning unless motion is reduced](images/scenes/rain.svg)

Clouds along the top row, drops at two depths (near ones as `|` falling fast, far ones as `:`
falling slower, with a slight wind), a puddle line along the bottom and a ripple where each near
drop lands, growing from `o` to `(   )` over 0.6 seconds. Every 5 to 15 seconds a bolt of lightning
is drawn for 0.12 seconds as one jagged line in the upper third of the screen. Every strike waits
for five seconds since the last, so a resize, which clears a bolt on screen, never brings the next
one early. It is a shape in the
corner of the eye, never a whole-screen flash. The gap between bolts is never under five seconds
whatever the random numbers say (`MIN_LIGHTNING_GAP`).

- Colours: near drops in bright blue, far drops in blue, ripples in cyan, the puddle line in the
  muted colour, clouds in grey and the bolt in bright yellow.
- Small screens: a screen under 7 columns or 6 rows gets no bolt.
- Reduced motion: no lightning at all and rain at half speed.
- Tests: drops and ripples are drawn, no bolt ever appears under reduced motion, the gaps between
  bolts are at least five seconds over five simulated minutes, also with a new size on every frame;
  a tiny screen never gets one.

### The storm

![The storm scene: Heavy slanting rain under two rows of cloud, with distant lightning unless motion is reduced](images/scenes/storm.svg)

The watch mode shows a storm while a build fails. It is the rain scene with twice the drops, a
stronger wind that slants the near drops to `/` and a second, offset row of cloud below the first.
Its bolts come every 5 to 9 seconds instead of 5 to 15, with the same five second minimum gap, and
the first bolt waits five seconds from the moment the storm appears, so the change to a storm never
opens with a flash. Since 0.3.0 it is a scene of its own, `storm`, so it runs on its own and can go in the fail list.

- Colours: the same as the rain.
- Reduced motion: no lightning at all and rain at half speed.
- Tests: the gaps between bolts, the first bolt and no bolt under reduced motion over five
  simulated minutes; twice the drops, the second row of cloud and the slanted drops; every colour
  at four sizes in every theme.

## snow

![The snow scene: Drifting flakes that settle at the bottom](images/scenes/snow.svg)

Flakes at two depths (`*` near, `.` far) drift down with a slow sideways sway and settle where
they land. Three flakes raise a column's bank by one row and a column that stands more than a row
above its neighbour sheds into it, so the bank stays rounded. The bank is drawn in full blocks with
a half block on top for a half row and never rises above a third of the screen.

- Colours: near flakes in the text colour, far flakes in the muted colour, the bank in the text
  colour on a dark theme and in grey on a light theme, so it does not become a slab of black.
- Reduced motion: flakes fall and sway at half speed.
- Tests: flakes fall and a bank forms within forty seconds; the bank never passes a third of the
  screen over ten simulated minutes.

## starfield

![The starfield scene: Twinkling stars, a crescent moon and a slow shooting star](images/scenes/starfield.svg)

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

![The code-rain scene: Falling columns of characters in the theme's syntax colours](images/scenes/code-rain.svg)

Every column of the screen is a stream of characters (letters, digits and symbols, all ASCII) that
falls from the top at its own speed, with a bright head and a tail that fades through the column's
colour into grey. Characters in a falling column change now and then. A column that has fallen off
the bottom waits a while before falling again; the wait and the speed both scale with the height
of the screen so a short terminal stays full and a tall one is not a blur.

- Colours: each column takes one of the theme's nine syntax colours, the head is the strongest
  text colour and the last two characters of the tail are grey.
- Reduced motion: columns fall at 1.5 to 3 rows a second (`CALM_MAX_SPEED`) and a column's
  characters are never swapped. The trail still scrolls, so a cell shows a new character each time
  the column moves down a row: at most three times a second.
- Tests: columns fall in syntax colours with a bright head; under reduced motion a column's
  characters stay the same while it falls and no cell changes more than three times in any second.

## fireflies

![The fireflies scene: A dark meadow with slow pulsing lights](images/scenes/fireflies.svg)

A dark meadow: a row of grass tufts along the bottom with a few taller stalks above it, plus
lights that wander slowly with a gentle random walk, bouncing off the edges and staying above the
grass. Each light pulses on its own cycle of 2.5 to 5 seconds, from not drawn through `.` and `o`
to `*`.

- Colours: grass in green, lights in yellow and, at their peak, bright yellow.
- Reduced motion: cycles take 5 to 9 seconds and the lights wander at under half the speed.
- Tests: no light has a cycle shorter than `MIN_PULSE_PERIOD`; the lights reach every brightness;
  they stay inside the screen and above the grass.

## aurora

![The aurora scene: Slow waves of colour across the top of the sky](images/scenes/aurora.svg)

Two or three bands of colour across the upper half of the sky, each a slow wave made of two sine
waves whose phase drifts at a different speed per band, drawn in three shades (`▓`, `▒`, `░`) from
the band's centre outwards; on the High Contrast and Deep Contrast themes the faintest shade is `▒`.
Their thickness breathes gently, once every 21 seconds. Faint stars
sit behind the bands and a mountain ridge, drawn as a single outline of `/`, `\` and `^`, runs
along the bottom.

- Colours: the bands in green, cyan, magenta and blue, stars in the muted colour, the ridge in
  grey.
- Small screens: two bands rather than three under 10 rows.
- Reduced motion: the waves drift and breathe at half speed, one breath every 42 seconds.
- Tests: the shaded cells are in the upper half and none in the lower, the picture changes over
  time, the ridge is drawn and the calm bands move at exactly half the speed.

## campfire

![The campfire scene: Flames and embers with a steady glow](images/scenes/campfire.svg)

Logs drawn as `\=====/` on the ground with a steady glow of dots either side, a flame above them
built row by row from a base about a quarter of the screen wide, narrowing towards the top, with
three layers (`▓` core, `▒` middle, `░` edge, `▒` on the High Contrast and Deep Contrast themes)
and `^` tips. Each row of the flame takes a new lean and width every 0.6 seconds
(`FLICKER_PERIOD`), the rows on staggered timers so the fire never changes all at once. A row
switches straight to its new shape rather than blending, because a blend can cross a shade boundary
several times on the way; so no flame cell changes more than twice in any second. A resize keeps
the rows the flame still has. Embers rise from the top of the flame, drift and fade from `*`
to `.` over one and a half to three seconds.

- Colours: core in bright yellow, middle and logs and glow in yellow, edges, tips and the last
  stage of an ember in red.
- Reduced motion: each row changes every 1.5 seconds (`CALM_FLICKER_PERIOD`), so a cell at most once
  a second. Embers are fewer and slower.
- Tests: the logs, the core and the tips are drawn and embers appear; counted cell by cell over 30
  seconds at three sizes, no flame cell changes more than twice in a second (once under reduced
  motion); each row takes about one shape a period; a resize keeps the rows.

## waves

![The waves scene: A calm sea horizon with gulls](images/scenes/waves.svg)

A horizon at two fifths of the way down with a sun top right (on a screen at least 16 wide with at
least three rows of sky) and gulls crossing the sky, their wings moving through four positions over
a 1.6 second beat. Below the horizon every row is a wave of `~` crests and `-` troughs drifting
sideways, with longer and faster waves nearer the bottom and the occasional `'` of foam on the near
rows. The sun is drawn last, so a gull passes behind it.

- Colours: the horizon and far crests in cyan, near crests in bright cyan, troughs in blue, foam
  and gulls in the text colour, the sun in bright yellow.
- Reduced motion: the swell, its drift and the wing beats at half speed.
- Tests: the horizon row is unbroken, every row below it has waves, the sun and a gull are drawn at
  60 by 20 and the sun is drawn at 20 by 8 but not at 12 by 5. A wing beat changes under three times
  a second.

## zacess

![The zacess scene: The Zacess mark drifting slowly](images/scenes/zacess.svg)

The Zacess mark: a page with a folded top right corner and a band across its middle that reaches
past the page on both sides, drifting slowly across the screen and bouncing off the edges. The
page is picked from five sizes, the largest that fits with room to move, from 15 by 11 down to 5 by
5; a screen too small for any of them shows a still `Z`. A row of background above and below the
band, across the whole mark, keeps the band apart from the page and the tile, so it stays visible
where its colour is close to theirs and under `NO_COLOR`. When there is room the word `Zacess` sits
beneath the mark.

- Dark themes: the terminal background stands in for the mark's dark tile, the page is drawn in
  the text colour with `◣` cells for the fold, also in the text colour. The band is in bright
  yellow.
- Light themes: the tile is drawn in the text colour with its corners left off, the page is left as
  the background with `◥` cells for the fold and the band is in the theme's yellow.
- The Linux console and the classic Windows console fonts lack `◣` and `◥`, so there the fold is
  left as a square cut corner.
- Reduced motion: it drifts at half speed.
- Tests: the band never touches a cell of another colour in any theme at three sizes; without
  colour the band row is wider than the page with blank rows beside it; the page sizes chosen for 20
  by 8 and 120 by 40, the page, band and fold drawn in both
  appearances and the mark moving while staying inside a 20 by 8 screen for a simulated minute.

# Pass scenes

Calm scenes made for a passing build. Each has captions of its own for the watch mode.

## koi-pond

![The koi-pond scene: A garden pond from above, with koi gliding under lily pads and slow ripples](images/scenes/koi-pond.svg)

A garden pond seen from above. A few large koi glide across it on slow curves: each is a three
row fish with a rounded body, an eye at the front, two spots on its back and a tail that sways
between spread and closed, holding each position for 1.2 seconds (`TAIL_SWAY`). A koi that has
crossed the pond comes back in from either side. Lily pads are drawn over the koi, so a fish slips
underneath them: large ones (`(   <)` with a notch, some with a flower) and small `(_<)` ones.
Flecks of water drift on a slow current and every few seconds a ripple spreads where something
touched the surface, from `o` to `( )` to a wider ring and two last dots, one stage every 0.6
seconds (`RIPPLE_STAGE`). The far bank is a plain line of grass with a tuft here and there and a
rim of rounded stones runs along the bottom.

- Colours: koi in four coats (bright red with spots in the text colour, bright yellow with red, the
  text colour with bright red and yellow with the text colour); pads in green and bright green with
  flowers in bright magenta and magenta; flecks in blue; ripples in cyan fading to the muted colour;
  the bank in green and bright green; the rim in grey.
- Small screens: below 40 by 12 each koi is a one row fish (`><(*o)`); only small pads below 30
  columns.
- Reduced motion: koi swim and curve at half speed with their tails still, ripples come every 5 to
  10 seconds instead of 2.5 to 6 and each ring lasts twice as long.
- Tests: the ripple stage and the tail sway never drop below their constants at any motion setting;
  koi stay in the water over two simulated minutes at the lively level; pads, a koi and the rim are
  drawn.

## harbour

![The harbour scene: A harbour town at night, with moored boats, quay lamps and their reflections](images/scenes/harbour.svg)

A small harbour town at night. A row of one to four storey buildings with lit windows stands back
behind a promenade of lamps, above a stone quay edge (`=`). The water is mostly dark and still,
with a few short swell lines on every third row drifting slowly. Each lamp throws a straight column
of light (`|` and `:`) down the water, broken here and there with a short glint either side near
the top. The column shimmers on a slow beat, one change every 0.8 seconds (`SHIMMER_PERIOD`). Moored sailing boats, cabin boats and rowing boats sit at several depths. A
sailing boat's mast sways through four positions over four seconds (`SWAY_PERIOD`) with a riding
light at its top. The water laps either side of each hull in time with it. A lighthouse with a
steady lamp stands on the breakwater to the right. A crescent moon and stars hang over the town.
Now and then a ferry with lit windows crosses the far water.

- Colours: buildings and the quay in grey, windows in yellow, lamps and their nearer reflections in
  bright yellow and the rest in yellow, boats in the text colour, bright cyan or bright red, lapping
  water in cyan, the swell in blue and the muted colour, the lighthouse in the text colour with red
  bands, stars in the text and muted colours.
- Small screens: the lighthouse needs 50 columns and six rows above the quay, the moon 40 by 7 rows
  of sky and the ferry 40 columns and eight rows of water. A short screen gets rowing boats only.
- Reduced motion: masts stay upright and the water stops lapping. The shimmer changes half as
  often and the ferry crosses at half speed.
- Tests: the shimmer and sway periods hold at every motion setting; the quay row is unbroken, the
  lamps and the lighthouse are drawn and a reflection reaches the water; at 20 by 8 the quay is
  still there and the lighthouse is left out.

## lanterns

![The lanterns scene: Paper lanterns rising slowly over a dark lake, with their reflections on the water](images/scenes/lanterns.svg)

Paper lanterns rise slowly from a dark lake into a starry sky at three depths: far ones as a
single `o`, middle ones as `(o)` and near ones as a three row lantern with a glowing centre. Each
sways one column either way over seven seconds (`SWAY_PERIOD`) and leaves a reflection on the water
as far below the shore as it is above it, squeezed by half, so the reflection slides down as the
lantern climbs. A lantern starts behind the far treeline and comes into view as it clears it. A few
more lanterns float on the surface, drifting slowly to the right.

- Colours: lanterns in bright yellow, yellow, bright red, red and bright magenta, each reflection in
  the dimmer shade of its lantern; the treeline in green; the swell in blue; stars in the text and
  muted colours.
- Small screens: near lanterns need 30 by 14.
- Reduced motion: lanterns rise, sway and drift at half speed.
- Tests: lanterns rise and the sway period holds at every motion setting; the treeline and a
  lantern are drawn; no near lantern at 20 by 8.

## balloons

![The balloons scene: Hot air balloons lifting slowly from behind rolling hills under drifting clouds](images/scenes/balloons.svg)

Hot air balloons in three sizes lift slowly from behind three ridges of rolling hills and drift on
a light wind. Small balloons (`(|)`) rise from behind the far hills, medium ones from behind the
middle hills and large ones, with gores, ropes and a basket, from behind the near hills, so each
hill hides the balloons that belong behind it. Each envelope is solid, so nothing behind shows
through it. Clouds drift slower still and the nearer hills carry tufts of grass and a few trees.

- Colours: envelopes in bright red, bright yellow, bright cyan, bright magenta or bright green with
  gores in a second colour, baskets in yellow, ropes in the muted colour; the far ridge in blue,
  the middle in green and the near in bright green, with tufts and trees in green; clouds in the
  muted colour.
- Small screens: large balloons need 40 by 24 and medium ones 12 rows; at 20 by 8 only small ones
  rise. Each hill rises and falls by at least a row, so a short screen still has hills.
- Reduced motion: balloons rise and drift at half speed and clouds move at half speed.
- Tests: balloons rise and the hills stay put; the fastest balloon climbs under a row a second at
  the lively level; the sizes chosen for 100 by 30 and 20 by 8.

## countryside

![The countryside scene: A steam train crossing open country, with hills, fields, trees and a fence sliding past](images/scenes/countryside.svg)

A steam train crossing open country, seen from beside the line. The view keeps pace with the train,
so the land slides past in layers at different speeds: far hills with trees on the crest slowest,
then fields with lines of hedgerows, each row a little faster than the one above it, then oaks and
pines beside the line, the track with its sleepers at 2.5 columns a second (`TRACK_SPEED`) and a
fence right in front, fastest of all. The scenery is worked out from where it is in the world, so
it never repeats and needs nothing stored. The train is an engine pulling up to four carriages with
lit windows. It eases forward and back a little over a minute. Smoke puffs from its chimney,
drifting back and up and shrinking from `O` to `o` to `.` one size a second (`PUFF_STAGE`).

- Colours: carriages in bright red and bright blue, the engine in the text colour, windows in bright
  yellow; the far hills in blue, crest trees, hedgerows and tufts in green, the trees by the line
  in green and bright green with trunks in yellow; the track in grey and the fence in yellow; smoke
  in the text and muted colours; the sun in bright yellow and clouds in the muted colour.
- Small screens: carriages fill up to seven tenths of the width, so at 20 columns only the engine
  runs. Tall trees need 18 rows and the sun 40 by 14.
- Reduced motion: every layer slides at half speed and smoke puffs half as often and shrinks half
  as fast.
- Tests: the train stays wholly in view over two simulated minutes at the lively level while the
  land moves on; the engine, carriage windows, track and fence are drawn; the puff stage holds; the
  engine alone at 20 by 8.

## winter-cabin

![The winter-cabin scene: A log cabin in the snow at night, with a lit window, chimney smoke, pines and falling snow](images/scenes/winter-cabin.svg)

A log cabin in a snowy clearing at night: snow on its roof, a lit window whose light spills onto
the snow in front and smoke curling from the chimney, leaning with a light breeze and shrinking
from `o` to `O` to `.` one stage every 1.2 seconds (`PUFF_STAGE`). Pines drawn in tiers of
branches, with snow resting on them, stand along the edge of the clearing, with two tall ones in
front at the sides. Snow falls gently at two depths and melts into the snowy ground. A crescent
moon and stars hang above the trees.

- Colours: the roof and the snow's surface in the text colour, walls and door in yellow, the window
  in bright yellow and its light on the snow in yellow, the chimney in grey; pines in green and
  bright green with trunks in yellow; smoke in the text, grey and muted colours; snow in the text
  colour; stars in the muted colour and the moon in bright yellow.
- Small screens: below 40 by 16 a smaller cabin with a lit window; the tall pines need 30 by 10 and
  the moon 40 by 14.
- Reduced motion: snow falls and sways at half speed, smoke rises half as fast and puffs half as
  often.
- Tests: the window is lit, smoke rises above the chimney and the puff stage holds; snow never falls
  below the ground over thirty seconds at the lively level; the small cabin at 20 by 8.

## autumn

![The autumn scene: A tree in autumn colour dropping leaves that drift down and gather on the ground](images/scenes/autumn.svg)

A tree in full autumn colour: a trunk with roots and a branch up to each of several rounded
clusters of leaves, one on top and others either side. The canopy has a clear outline (`(` and `)`
at its sides, `~` along the top and `"` underneath) and is filled with `@` leaves, with a few gaps
where the branches show through. A smaller second tree stands on a wide screen. A few leaves at a
time let go and drift down on the breeze as `<>`, tumbling through `</` and `\>` with each turn
lasting 0.6 seconds (`LEAF_TURN`). They settle into a pile along the ground that stays rounded (a
leaf landing on a heap slides to the lower side) and never grows past two rows. Now and then a
skein of geese crosses the low sky in a V, past a low sun and a line of far hills.

- Colours: each cluster of leaves in red, yellow or bright yellow, with the falling and fallen
  leaves in the same three; the trunk and branches in yellow; grass in green; geese in the text
  colour; the far hills in blue; the sun in bright yellow.
- Small screens: the pile is one row deep below 12 rows; geese need 30 by 12, the sun 40 by 14 and
  the second tree 60 by 14.
- Reduced motion: leaves fall, drift and come loose at half the rate and fall without turning.
- Tests: the turn period holds at every motion setting; over ten simulated minutes leaves settle
  and the pile never passes its cap; the tree is drawn at 20 by 8.

## blossom

![The blossom scene: A flowering branch letting petals go on a breeze, down to a stream that carries them off](images/scenes/blossom.svg)

A flowering branch reaches in from the top left corner and droops across the sky as one clear
line of `_` and `\`, with short twigs up or down every few columns. Each twig ends in a flower: a
bright centre (`o`) with four petals (`*`) around it, kept apart from its neighbours. A few petals
at a time let go on a spring breeze and tumble slowly down, turning over every 0.7 seconds
(`PETAL_TURN`), past far hills and a meadow to a stream that carries them off to the right. A
small bird sits on the branch and turns its head every six seconds (`BIRD_LOOK`).

- Colours: petals in bright magenta, magenta or the text colour, one colour to a flower, with the
  centres in bright yellow; the branch in yellow; the bird in bright cyan; the stream in cyan and
  blue; the bank and meadow in green with stones in grey; the far hills in blue.
- Small screens: the bird needs 30 by 10. Below 10 rows the stream is one row and the hills and
  meadow are left out.
- Reduced motion: petals fall, drift and float at half speed without turning, fewer come loose and
  the bird looks about half as often.
- Tests: the petal and bird periods hold at every motion setting; petals reach the stream and float
  away; blossom, the bird and the stream are drawn; blossom at 20 by 8.

## city-night

![The city-night scene: A night skyline where windows light and darken one at a time, a moon, a river and a passing train](images/scenes/city-night.svg)

A night skyline in two layers: a far row of buildings drawn as a flat shaded silhouette and a near
row drawn as outlines with a grid of windows, some with an antenna and a small red light on top.
One window at a time lights up or goes dark, every 1.2 seconds (`WINDOW_PERIOD`). A crescent moon
hangs in a sky of faint stars. A river runs along the bottom with a few lit windows reflected in
it. On a screen with room a bridge carries a train of three to five carriages across now and then,
every 15 to 35 seconds.

- Colours: stars in the muted colour, the moon in bright yellow, the far skyline in grey, the near
  buildings in the muted colour, lit windows in yellow and dark ones in grey, the river in blue
  with reflections in yellow, the bridge in the muted colour and grey, the train in the text colour
  with bright yellow windows and the antenna light in red.
- Small screens: the moon needs 30 by 12, the river 10 rows and the bridge and train 30 by 14.
- Reduced motion: windows change half as often and the train runs at half speed and comes every 30
  to 60 seconds.
- Tests: at most one window changes at a time and no more often than `WINDOW_PERIOD` at any motion
  setting; a train crosses an 80 by 24 screen within a minute and never a 20 by 8 one.

## orbit

![The orbit scene: A ringed planet among distant stars with a small moon on a slow orbit around it](images/scenes/orbit.svg)

A banded planet lit from the upper left, drawn in shades rather than solid blocks: three levels of
light across its day side (`▓`, `▒` and the faint shade), alternate bands a level fainter, a line of
`:` along the terminator and only a faint rim on its night side, with a slow storm crossing its
face once a minute. A ring circles it as one continuous ellipse, its flat stretches drawn as `-` or
`_` by where the curve sits in each cell, its slopes as `/` or `\` and its ends as `(` and `)`. It
passes behind the planet at the top and in front of it at the bottom.
A small moon goes round once every 40 seconds (`ORBIT_PERIOD`), passing in front of the planet and
hiding behind it. Distant stars fill the sky.

- Colours: the planet's bands in yellow, bright yellow and red with its terminator and night rim
  in grey, the ring in the muted colour, the moon in the strongest text colour and stars in the text and muted
  colours.
- Small screens: the planet's size follows the screen, down to a radius of a row or two at 20 by 8;
  the moon is drawn as `o` on a small planet and `O` on a large one.
- Reduced motion: the moon and the storm move at half speed, one orbit every 80 seconds.
- Tests: the moon turns well under a tenth of a turn a second even on lively and spends time both
  in front of and behind the planet; the planet is drawn in shades with no solid block and the
  ring and stars are drawn at every size.

## constellations

![The constellations scene: Stars that are slowly joined into named figures one group at a time, then fade](images/scenes/constellations.svg)

A sky of faint stars with brighter ones in groups. One group at a time is joined into a figure: its
lines are drawn at six cells a second and the finished figure is held for six seconds. Each line is
a straight segment walked one cell a step, each cell drawn by the step that reached it: `|`
straight down, `/` or `\` on a diagonal, `_` along the flat of a sloping line and `-` on a level
one. The figure's name, such as The Kettle or The Heron, shows for as long as the figure does. It sits
beneath the figure unless the figure is at the bottom of the screen, where it sits above. It then fades in two steps 1.2 seconds apart
(`FADE_STEP`) and the sky rests for a moment before the next group. Every name is our own.

- Colours: faint stars in the muted colour, the stars of every group in the text colour, the
  joined stars in bright yellow and lines in cyan, then bright cyan while held, then grey as they
  fade. The name is in the text colour while the figure is held and the muted colour while it is
  drawn or fades.
- Small screens: the sky is split into one box per group, so a 20 by 8 screen has one small
  figure.
- Reduced motion: lines are drawn at half speed and the hold and fade steps take twice as long.
- Tests: figures are drawn, named and faded one at a time over two minutes; every line is one
  cell a step with no cell doubled on a row or a column and reads the same drawn from either end;
  a small sky still has a figure.

## tea

![The tea scene: A mug of tea with curling steam beside books and a plant, with weather at the window](images/scenes/tea.svg)

A mug of tea on a table, steam curling up from it in small `(` and `)` puffs that sway as they
rise and thin to dots before they go. Beside the mug sit a stack of three books and a small potted
plant and above them hangs a lamp. Behind, a window with a cross bar shows rain or snow, picked at
the start, falling past a far roofline.

- Colours: the mug in cyan with the tea in yellow, steam in the muted colour then grey, the table
  in yellow, the lamp cord in grey, the lamp in bright yellow, the books in red, blue and green,
  the plant in green with a red pot, the window frame in yellow, rain in blue, snow in the text
  colour and the roofline in grey.
- Small screens: below 40 by 14 the mug is a smaller one and the books, plant and table legs are
  left out; the window needs 30 by 12 and the lamp needs room beside the window.
- Reduced motion: steam rises and sways at half speed with fewer puffs and the weather falls at
  half speed.
- Tests: a puff's character changes at most twice a 2.4 second sway (`SWAY_PERIOD`) at every
  motion setting; steam rises above the mug; the window needs room.

## cat-nap

![The cat-nap scene: A cat asleep on a cushion, breathing slowly, with drifting z's, wool on the rug and a night window](images/scenes/cat-nap.svg)

A cat asleep on a cushion, its eyes shut. Its back rises and falls with each four second breath
(`BREATH_PERIOD`) and its tail stirs once every seven seconds (`TAIL_PERIOD`). A `z` drifts up from
it every few seconds, growing to `Z` and thinning as it goes. A night window with a crescent moon
and stars hangs on the wall and a ball of wool lies on a patterned rug. On a wide screen a
bookcase stands against the right wall.

- Colours: the cat in the text colour with a red nose, the cushion in magenta, the z's in the
  muted colour then grey, the rug in red and yellow, the wool in cyan, the window frame in the
  muted colour with stars in the text colour and the moon in bright yellow, the sill and bookcase
  in yellow with spines in six colours.
- Small screens: the window needs 36 columns, the wool 40 and the bookcase 70 by 16.
- Reduced motion: breaths, the tail and the z's all take twice as long and the z's drift at half
  speed.
- Tests: the back changes shape once or twice a breath and the breath and tail periods are never
  shorter than their constants; a z drifts up from the cat; the cat and both ends of its cushion
  fit a 20 by 8 screen.

## garden

![The garden scene: Seeds that sprout, grow and bloom over a couple of minutes, with butterflies, before the bed starts again](images/scenes/garden.svg)

A bed of tulips, daisies, sunflowers, bluebells, ferns and roses in front of a picket fence, under
a sun and a slowly drifting cloud, with butterflies wandering above. Over a 110 second season
(`SEASON`) each seed sprouts, its stem grows a row at a time with leaves on alternating sides, a
bud forms and the flower blooms. Near the end of the season the petals fall, the stems settle back
into the soil and the bed starts again. The first season opens part grown, so the bed is never
bare when the scene appears. Each plant's state comes from the season's clock, so a
resize keeps the season where it was.

- Colours: stems, leaves, grass tufts and buds in green, soil in yellow, the fence and cloud in
  grey, the sun in bright yellow, tulips in red, daisies in the text colour with yellow centres,
  sunflowers in bright yellow with red centres, bluebells in blue, roses in magenta and the
  butterflies in magenta and bright cyan.
- Small screens: the sun and cloud need 30 by 12, the fence 12 rows and the butterflies 10 rows.
- Reduced motion: the season takes twice as long, wings beat half as often and the cloud and
  butterflies move at half speed.
- Tests: a season grows, blooms, sheds its petals and starts again; a stem takes at least a second
  a row; a butterfly's wings change at most twice a 1.2 second beat (`FLUTTER_PERIOD`).

## tiles

![The tiles scene: A grid of small folded pages in the theme's colours, one tile changing at a time](images/scenes/tiles.svg)

A grid of small pages, each with the Zacess mark's folded corner. About a third are filled in one
of ten colours from the theme and the rest are drawn as faint outlines with a dot for the fold.
Every few seconds one tile takes a new colour, fills or settles back to its outline: every three
seconds on a small grid (`CHANGE_PERIOD`) and never more often than every 0.7 seconds on a large
one (`MIN_CHANGE_GAP`). The grid drifts back towards a third filled, so it never becomes a wall of
solid colour.

- Colours: filled tiles in the accent colour, cyan, magenta, green, yellow, blue, bright cyan and
  the keyword, string and function colours; outlines in grey.
- Small screens: tiles are four by three cells, six by four from 120 by 40; a margin is kept on
  screens of 40 by 12 and more.
- Reduced motion: tiles change half as often.
- Tests: at most one tile changes at a time and never more often than `MIN_CHANGE_GAP` at every
  size and motion setting; the grid stays between a sixth and three fifths filled over ten
  minutes.

## ribbons

![The ribbons scene: Broad ribbons of contour lines flowing slowly across a faint grid of dots](images/scenes/ribbons.svg)

One to five broad ribbons, each a solid centre line between two dotted contours, flowing slowly
across a faint grid of dots. Each ribbon is two slow waves added together, so its shape keeps
changing. No crest travels sideways faster than four columns a second (`MAX_DRIFT`).

- Colours: the ribbons in cyan, magenta, green, yellow and blue and the dots in the muted colour.
- Small screens: below 16 rows each ribbon is its centre line alone.
- Reduced motion: the ribbons flow at half speed. Lively adds a ribbon.
- Tests: no crest drifts faster than the limit at any motion setting; every ribbon crosses the
  whole screen.

# Fail scenes

Slow, serious scenes made for a failing build, like the storm above. Each progresses for about a minute and then holds or eases into a gentle loop, so it still looks right after an hour. None flashes, shakes or spins.

## flood

![The flood scene: A street where the water rises slowly past the doors, with debris drifting on the surface](images/scenes/flood.svg)

A street of houses (red roofs, walls and doors in the muted colour, lit windows as `[]`) under one
row of drifting cloud and a light drizzle. Over the first 45 seconds (`RISE_SECONDS`) the water
rises from the road, easing in and slowing as it nears its height: about half the height of the
lowest house, so the doors and lower windows go under while the roofs and the sky stay clear. It
then holds with a slow swell of a third of a row every 12 seconds (`SWELL_PERIOD`). The surface is
a drifting line of `~` and `-`, with a scattering of swell marks below it and anything under the
water redrawn in blue. Debris (`[#]`, `(=)`, `===`, `[_]`, `\_/`, `o`) drifts both ways along the
surface. A warning sign `/!\` stands in a clearing of its own, its post going under. A resize keeps
the water at the height it has reached.

- Colours: roofs in red, walls and the road in the muted colour, windows in yellow, the sign in
  the warning colour, the surface in cyan, everything under water in blue, debris in yellow, the
  text colour and magenta, clouds in grey and the drizzle in blue.
- Small screens: the sign is left out under 24 by 10; the houses are narrower under 40 columns.
- Reduced motion: the water rises, swells and drifts at half speed and the drizzle is halved.
- Tests: the water starts below the doors, reaches its height and stays within a row of it for
  the last minute of a simulated hour, never above the third row; the surface row moves at most
  twice a swell even when lively; a resize keeps the water risen.

## listing-ship

![The listing-ship scene: A cargo ship on a grey sea that slowly lists to one side as its waterline creeps up](images/scenes/listing-ship.svg)

A cargo ship drawn as a solid silhouette of half blocks (hull, bridge, a row of portholes and a
mast with a flag) on a grey sea under two rows of cloud and a drizzle. Over the first 50 seconds
(`LIST_SECONDS`) it settles by the bow to an angle of 8 degrees (`MAX_LIST`) and sinks a row: the
stern stays put and the bow goes down, the keel and deck stepping half a row at a time so the
shape never breaks apart. Whatever goes below the surface is drawn as a faint shade in blue, so the
waterline visibly creeps up past the portholes. It then rolls a degree and a half either way every
11 seconds (`ROLL_PERIOD`), its mast leaning with it. A lifebuoy `(O)` and a crate drift away
along the surface. On a wide screen the ship sits left of the middle, so the watch mode's panel
leaves it in view.

- Colours: the hull in the text colour, the bridge in the muted colour, portholes in yellow, the
  mast in grey, the flag and the lifebuoy in the error colour, anything below the surface in blue,
  the sea in grey and blue, clouds in grey.
- Small screens: a 16 column ship under 50 by 16, a 36 column one above.
- Reduced motion: it lists, rolls and drifts at half speed and the drizzle is halved.
- Tests: the angle never changes faster than two degrees a second, so the bow moves under a row a
  second; it holds within the roll of its final angle for the last 100 seconds of a simulated
  hour; the flag, hull and portholes are drawn at 100 by 30 and the flag at 20 by 8.

## corrosion

![The corrosion scene: Riveted steel plates where rust creeps out from the rivets cell by cell and slowly drips](images/scenes/corrosion.svg)

A wall of riveted steel plates, every other row offset by half a plate, with seams of `-`, `|`
and `+` and a line of rivets beside each seam. Rust starts at a few rivets and creeps cell by cell,
most readily downwards and along the seams, with a new patch now and then, until it covers half
the wall (`MAX_COVER`) after about a minute; then it stops. Each rusty cell darkens through four
stages, `.`, `:`, `%` and a shade, one stage every 8 seconds (`STAGE_SECONDS`). Seams and rivets
in heavy rust take its colour. Drips `'` run slowly down from the darkest rust every few seconds.
A warning plate `/!\` sits in a cleared space near the top right. A resize stretches the rust onto
the new size, so the wall is never cleaned.

- Colours: seams in grey, rivets in the muted colour, rust from yellow through the warning colour
  and red to the error colour, drips in red, the plate in the warning colour.
- Small screens: the warning plate is left out under 30 by 10.
- Reduced motion: rust spreads and drips at half speed, drips come half as often and each stage
  takes twice as long.
- Tests: under a tenth of the wall is rusty after five seconds, most of the cover after two
  minutes and never more than half after an hour; the stage only depends on age, so a cell changes
  at most four times in its life and no faster than once every 8 seconds; a resize keeps the rust.

## tangle

![The tangle scene: Cables hanging from a patch panel that slowly wander, cross and knot into a snarl](images/scenes/tangle.svg)

A patch panel along the top row with a port `[o]` for each cable, the cable's colour in its port.
At first every cable hangs straight down. Over the first 45 seconds (`TANGLE_SECONDS`) their lower
ends wander across their neighbours and each cable takes on a slow wave, so they cross, loop and
knot (`@`) until they hang in a snarl that sways slowly. A steep run is drawn as a short level
stretch of `_` ending in a slant, so a cable never breaks. Loose ends lie coiled on the floor as
`(@)` and a warning tag `-[! tangled]` (or `-[!]` on a narrow screen) hangs off one cable a third of
the way down. Every cable end stays inside the screen.

- Colours: cables in blue, green, magenta, cyan, yellow and red, the panel and floor in grey,
  ports in the muted colour, the tag in the warning colour.
- Small screens: the tag is left out under 16 columns or when there are fewer than four rows of
  cable.
- Reduced motion: the cables tangle and sway at half speed.
- Tests: every cable hangs straight at first and tangles fully within an hour, staying on screen;
  the tangle never grows by more than about a thirtieth of the whole in a second.

## wilt

![The wilt scene: Potted plants on a sunny sill whose stems droop, leaves fall and soil cracks](images/scenes/wilt.svg)

Potted plants in a row on a windowsill, a hard sun in the top left corner. Each has a stem with
leaves `<` and `>` on alternate sides and a flower `@` at the tip. Over the first 50 seconds
(`WILT_SECONDS`) the stems bend further and further, each plant by its own amount, until they arch
over and the heads hang below where they started. The stems turn yellow and the flowers fade from
`@` to `o` to a dry `.`. The leaves dry to `,` and fall one by one, drifting down to lie on the
sill. The soil `.` in each pot cracks into `v`, `/` and `\`. The drooped stems then sway slowly
every 9 seconds (`SWAY_PERIOD`). Each stem is drawn cell by cell from its continuous direction, so
a curve reads as one line.

- Colours: stems and fresh leaves in green, turning yellow as they dry, flowers in magenta,
  yellow, red and cyan, dry heads in the muted colour, pots in red, soil and fallen leaves in
  yellow, the sill in the muted colour and the sun in bright yellow.
- Small screens: the sun is left out under 30 by 14 and plants sit closer together under 40
  columns.
- Reduced motion: the plants wilt, sway and drop their leaves at half speed.
- Tests: every tip ends at least two rows lower than it started, every leaf lands on the sill and
  the soil cracks within a simulated hour; the stem characters follow the bend.

## fog

![The fog scene: A road where a fog bank rolls in and the signs along the verge fade from view](images/scenes/fog.svg)

A country road (a dashed yellow centre line between two edges) with grass along the verge, posts
carrying signs (`[SLOW]`, `[ 30 ]`, `/!\`, `[<<<]`, `[BEND]`) and lamps, pine trees between them
and a far ridge. A fog bank rolls in from the left over the first 50 seconds (`ROLL_SECONDS`), its
front soft over a quarter of the screen. Thin fog greys out what is behind it and adds a sparse
haze of `.`; thick fog hides it under `.` and `:` and, where it is thickest, a shade. The fog's
texture is smooth noise drifting at 1.2 columns a second (`DRIFT`) and lies thickest near the
ground. Once it has rolled in it hides about a third of the screen and keeps drifting, with the odd
gap where a sign shows through. The haze uses a fixed scatter per cell, so no cell flickers as the
fog moves over it.

- Colours: the road edges in the muted colour, the centre line and lamps in yellow, grass and
  trees in green, posts and the ridge in grey, the `/!\` sign in the warning colour and the other
  signs in the text colour. The fog and anything it greys out are in grey.
- Small screens: the ridge is left out under 10 rows and the trees under 14.
- Reduced motion: the fog rolls in and drifts at half speed.
- Tests: the fog starts clear with signs showing, never covers more than about a tenth of the
  screen in a second while it rolls in, hides over a quarter of it after an hour and never shows
  more signs than at the start; the noise is smooth and in range.

## broken-bridge

![The broken-bridge scene: A bridge with its middle span gone, girders hanging, a barrier and a waiting car over a river](images/scenes/broken-bridge.svg)

A bridge over a river with its middle span gone. The deck (`=`) and its railing (`+---+`) stand on
pairs of piers between grass-topped banks that slope down to the water. They stop either side of a
gap about a sixth of the screen wide. Girders hang from both broken ends (`\` and `/`), the broken
ends are marked `%` and the fallen span lies in the river below as `/====\`. On the road before
the gap a barrier `|X|` stands under a `/!\` sign, with `BRIDGE OUT` above it on a wide enough
screen and a car waiting behind it. The river drifts downstream from left to right as two waves of
different lengths, so it never settles into stripes. Every four seconds a fragment drops from one
of the broken ends and splashes, first as `o` and then as `( )`.

- Colours: the deck in the text colour, the railing and piers in grey, the banks in green with
  shaded earth in the muted colour, the hanging girders, the broken ends and the sign in the
  warning colour, the barrier and `BRIDGE OUT` in the error colour, the car in bright blue, the
  river in cyan and blue and the splash in bright cyan and cyan.
- Small screens: the car needs 40 columns and the `BRIDGE OUT` text 60. The sign needs at least
  three rows of sky.
- Reduced motion: the river and clouds move at half speed and fragments fall half as often.
- Tests: the deck has a gap with no deck in it, girders hang below both ends, the barrier, sign,
  text and car are drawn at 100 by 30 and a deck is still drawn at 20 by 8; fragments never fall
  more often than `FRAGMENT_PERIOD` at any level and each one splashes.

## leak

![The leak scene: A cracked pipe dripping into a bucket that slowly fills and overflows into a spreading puddle](images/scenes/leak.svg)

A pipe runs along the ceiling and down the right-hand wall, with joints (`#`) along it. In the
middle of the ceiling run it is cracked (`X`, with a small `\ /` above it) and drips fall from the
crack, one every 1.1 seconds (`DRIP_PERIOD`), into a bucket on the floor. Each drip raises the
water a little and lands with a small splash, so the bucket fills over about a minute. Once full,
water runs down its outside (`:`) and a puddle of `~` spreads along the floor a little with each
drip until it reaches its widest and holds there. A pressure gauge on the pipe sinks over two
minutes as its needle tips from `/` through `-` to `\`. A wet floor sign (`/!!\`, with
`WET FLOOR` on a wide screen) stands beside the bucket.

- Colours: the pipe in grey, the crack in the error colour, the bucket in the text colour, drips,
  the surface and the puddle in bright blue, the water below the surface in blue, the gauge needle
  in the text colour, then the warning colour, then the error colour as the pressure falls and the
  sign in the warning colour.
- Small screens: the gauge needs 24 columns, the sign 30 columns and 10 rows and its text 50
  columns.
- Reduced motion: drips come half as often and fall at half speed, so the bucket fills over two
  minutes.
- Tests: no level makes the drips quicker than `DRIP_PERIOD`; after 30 seconds the bucket is part
  full with no puddle and after an hour it is full with the puddle at its widest; a resize keeps
  the water.

## blackout

![The blackout scene: A night skyline whose windows go dark one by one beside a snapped power line, leaving candles](images/scenes/blackout.svg)

A city skyline at night beside a power pole with a snapped line. Buildings of different widths and
heights stand in a row along the street, each with a grid of windows. About two thirds of the
windows start lit (`#`) and they go dark (`.`) one at a time over about a minute and a half, in a
random order, never two in one step (`MIN_WINDOW_GAP`). Five seconds after some windows go dark a
candle (`i`) is lit in them. A few windows stay lit on their own power, so the city ends quiet but
not empty. The pole's line hangs from the crossbar (`\`) to a break marked `x`. Faint stars, a
crescent moon and slow clouds sit above. Once dark the city stays dark; only the clouds move.

- Colours: lit windows in yellow and bright yellow, dark windows in the muted colour, candles and
  the hanging line in the warning colour, the break in the error colour, buildings in grey, the
  pole and the moon in the text colour, stars in the muted colour and clouds in grey.
- Small screens: the pole needs 40 columns and 10 rows and the moon 30 columns and 12 rows.
- Reduced motion: the windows go dark at half the rate and the clouds drift at half speed.
- Tests: windows go dark one at a time until only the windows on their own power are lit;
  counted cell by cell over 30 seconds, no window cell changes more than twice in a second, even
  on the lively level; a resize keeps how dark the city is.

## traffic-jam

![The traffic-jam scene: Cars queued bumper to bumper before a red signal and a row of cones, exhaust drifting up](images/scenes/traffic-jam.svg)

A road along the bottom of the screen, with two lanes on a screen of 16 rows or more and one on a
smaller one. Cars (`,-=-.` over `(o___o)`) roll in from the left and ease to a stop behind a stop
line, bumper to bumper, each with a steady red brake light. Past the line stand two cones (`A`) in
each lane and a signal stuck on red: its lit lamp is `(O)` and the others are `(.)`, so it reads
without colour. A `ROAD CLOSED` sign stands beside it on a wide enough screen. Once the queue
reaches the left edge no more cars arrive. Stopped cars give a small exhaust puff every three
seconds (`PUFF_PERIOD`) that drifts up and back, fading from `o` to `.`. Low hills with a few
trees and slow clouds sit behind the road.

- Colours: cars in blue, cyan, magenta, green, bright blue, bright magenta and the text colour,
  brake lights, the lit lamp and `ROAD CLOSED` in the error colour, cones and the line under the
  sign in the warning colour, the lane divider in yellow, puffs and clouds in grey, the hills and
  trees in green and the kerbs in the muted colour.
- Small screens: one lane under 16 rows, the full signal from 24 columns (a single `(O)` below
  that), the sign from 40 columns and the hills from five rows of sky.
- Reduced motion: cars roll in at half speed and puffs come half as often and drift at half
  speed.
- Tests: after a simulated hour every car rests at its place with none past the stop line and
  the queue reaches the left edge; the signal, the sign and a car are drawn; no level makes the
  puffs quicker than `PUFF_PERIOD`.

## erosion

![The erosion scene: A cliff edge crumbling into the sea, stones tumbling down and a fence left hanging](images/scenes/erosion.svg)

A cliff above the sea crumbling slowly. Grass runs along the clifftop above shaded rock, with a
rough face of `|`, `/` and `\` dropping to the water. Every 2.2 seconds (`CRUMBLE_PERIOD`) a piece
breaks from the face and tumbles into the sea as an `o` that slides down the face and splashes.
The top and the bottom rows wear fastest. No row may stand more than two cells out from its
neighbours, so the face stays rough but continuous as it creeps back. A fence along the top stood
right up to the edge: as the ground goes, its posts are left hanging down the face (`\` and `+`).
A `/!\` sign reading `KEEP BACK` and a cottage stand well back from the edge. After a few minutes
the face reaches harder rock and stops wearing, with a pebble falling every seven seconds and the
waves still rolling in towards the cliff.

- Colours: rock in grey, the face in yellow, grass in green, the standing fence and the cottage in
  the text colour, the hanging fence and the sign in the warning colour, `KEEP BACK` in the error
  colour, stones in the muted colour, the sea in cyan and blue and splashes in bright cyan and
  cyan.
- Small screens: the cottage needs 50 columns, the sign enough room behind the fence and
  `KEEP BACK` a little more.
- Reduced motion: pieces break away half as often and fall, drift and splash at half speed.
- Tests: the edge has worn back within a minute; after an hour it has stopped without wearing
  past its limit and the fence hangs down the face; no level breaks pieces away quicker than
  `CRUMBLE_PERIOD`; a resize keeps the worn edge.

## drought

![The drought scene: Cracked earth spreading under a heavy sun, a dry riverbed with its last puddle and a bare tree](images/scenes/drought.svg)

Cracked earth under a heavy sun. The ground below the horizon is split into plates by a network of
cracks (`|`, `_` and `/`), each plate edge appearing whole at its own moment, so the cracks spread
edge by edge from about a third of them to all of them over a minute (`CRACK_SECONDS`). A dry
riverbed meanders across the ground between two banks, sand and pebbles (`o`) in it and the last
puddle (`~`) at its middle, which loses one cell every six seconds (`PUDDLE_SHRINK`) until it is
gone. A withered tree stands on the horizon, dust drifts across and a single bird glides past with
its wings held still. The sun is an outline with fixed rays and never shimmers. Once dry the scene
stays dry.

- Colours: the sun in bright yellow with its rays in the warning colour, the horizon, the cracks
  and the banks in yellow, the tree, the sand and the pebbles in the muted colour, the puddle in
  bright blue, dust in grey and the bird in the text colour.
- Small screens: a smaller sun without rays under 40 columns or 20 rows and none under 16 columns;
  the tree needs 30 columns and six rows of sky.
- Reduced motion: the cracks spread and the puddle dries at half the rate, dust and the bird move
  at half speed.
- Tests: after an hour more cracks show than at the start, the cracks are complete and the puddle
  is gone; at every level the puddle shrinks one cell at a time, never twice within
  `PUDDLE_SHRINK`.

# The watch mode

![The watch mode while a build passes](images/scenes/watch-pass.svg)

![The watch mode while a build fails](images/scenes/watch-fail.svg)

`zacess-scenes watch` draws a pass scene while waiting for the first result and while the build
passes and a fail scene while it fails: `waves` and `storm` unless the settings choose others, with
a fixed, cycling or shuffled rotation. The scene changes in one step when the result changes, with
no fade, flash or inverted frame between the two. A new scene starts from its own first frame.
The one exception is a fix: the pass scene opens out from the middle of the screen over the fail
scene in two seconds, its edge traced by a ring of small sparkles in green, bright yellow and bright
cyan, each fixed to its cell so it never twinkles. Under reduced motion it takes three and a half
seconds with no sparkles. `the_opening_never_flashes_and_ends_on_the_new_scene` runs it through
the flash meter in every theme. `n` and `b` change the scene at most once every 0.4 seconds. A user
scene that fails is swapped for the built-in scene of its kind and the reason shows in the panel
and the status bar.

Over the scene sits a panel cleared to the background, so the scene never runs through the text:

- the banner, five rows of `█` in the theme's green for ALL GOOD with a check mark or in its error
  colour for BUILD FAILED with a cross. A terminal narrower than the banner or under 13 rows gets
  a one line `✓ ALL GOOD` or `✗ BUILD FAILED` instead;
- below it, while passing, a short caption from the scene showing that changes with each passing
  run, then any warnings; just after a fix, a recovered caption; while failing, the scene's caption
  when there is room, then the errors and failing tests and the warnings after them. A user scene's
  hooks can offer the caption instead. Captions can be turned off. Each problem is
  its first line with its location indented beneath it. When they do not all fit, the last line
  says how many more there are;
- while waiting, a line saying which job bacon is running. If bacon stops on its own, its last line
  and how to start it again.

The key hint shows above the status bar for four seconds, then in the muted colour for one and a
half seconds, then not at all, as the scenes' help line does.

- Colours: the error and warning colours come from the theme. Where either is below the theme's
  floor, its red or yellow stands in. Captions and text are in the text colour and locations, the
  hint and the status bar's separators in the muted colour.
- Tests: `every_overlay_colour_keeps_the_floor_in_every_theme` draws every state in all eight
  themes and checks every colour. Other tests cover the status bar text at every width from 1 to
  120 columns, the banner sizes, the order of errors and warnings and the count of the rest.
