//! Checks shared by every scene's tests: the contrast, font, flash and size checks every scene
//! gets, the one every-theme check every pixel scene gets on top and the colour assertions the
//! scenes' own tests use.

use std::collections::HashMap;

use super::*;
use crate::catalogue::Catalogue;
use crate::cli::SnapshotState;
use crate::colour::{Rgb, xterm_colour};
use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
use crate::frame::Frame;
use crate::pixels::{IMAGERY_CEILING, distance, plain_glyph};
use crate::snapshot::Shot;
use crate::terminal::pixel_256;
use crate::theme::{self, ThemeData};

/// The sizes every scene must look right at: a phone-sized terminal, a classic terminal, a
/// full screen and a very wide display.
pub const SIZES: [(usize, usize); 4] = [(20, 8), (80, 24), (120, 40), (200, 60)];

/// Odd sizes a scene must survive without a panic, smaller than it is meant for included.
pub const ODD_SIZES: [(usize, usize); 6] = [(1, 1), (2, 2), (7, 3), (19, 7), (33, 9), (300, 90)];

/// Steps a scene `frames` times at 20 frames a second and returns the last frame.
pub fn render(scene: &mut dyn Scene, width: usize, height: usize, frames: usize) -> Frame {
    let mut rng = Rng::new(99);
    let mut frame = Frame::new(width, height);
    for _ in 0..frames {
        scene.step(0.05, &mut rng);
    }
    frame.clear();
    scene.draw(&mut frame, theme::default_theme());
    frame
}

/// Whether a character is safe in every terminal font the program supports: printable ASCII,
/// the middle dot, the block and shade characters, the light box drawing lines and the two
/// triangles of the Zacess fold, which have a fallback of their own.
pub fn safe_glyph(ch: char) -> bool {
    (' '..='~').contains(&ch)
        || ch == '\u{b7}'
        || ('\u{2580}'..='\u{259f}').contains(&ch)
        || "\u{2500}\u{2502}\u{250c}\u{2510}\u{2514}\u{2518}\u{251c}\u{2524}\u{252c}\u{2534}\u{253c}\u{25e3}\u{25e5}"
            .contains(ch)
}

/// Checks every colour in one drawn frame against each theme's floor and every character
/// against the safe set.
fn check_colours(scene: &dyn Scene, frame: &mut Frame, name: &str, size: (usize, usize)) {
    for theme in theme::all() {
        frame.clear();
        scene.draw(frame, theme);
        assert!(
            frame.drawn() > 0,
            "{name} drew nothing at {}x{} in {}",
            size.0,
            size.1,
            theme.name
        );
        for colour in frame.colours() {
            let ratio = theme.contrast_of(colour);
            assert!(
                ratio >= theme.floor(),
                "{name} draws {} at {ratio:.2}:1 on {}, below its {}:1 floor",
                colour.hex(),
                theme.name,
                theme.floor()
            );
        }
        // text drawn on a background of its own keeps the floor against that background
        for cell in frame.cells().iter().filter(|c| !c.is_pixel()) {
            if let (Some(fg), Some(bg)) = (cell.colour, cell.bg) {
                assert!(
                    fg.contrast(bg) >= theme.floor(),
                    "{name} draws {} on {} in {}, below its {}:1 floor",
                    fg.hex(),
                    bg.hex(),
                    theme.name,
                    theme.floor()
                );
            }
        }
    }
    for cell in frame.cells() {
        assert!(
            safe_glyph(cell.ch),
            "{name} draws {:?}, which some terminal fonts lack",
            cell.ch
        );
    }
}

/// Every colour a scene draws, in every theme, keeps that theme's contrast floor on its
/// background: every frame of a minute at 80 by 24 at the standard level, long enough for
/// lightning, shooting stars and the whale, plus sampled frames at every other size and
/// motion setting. The scene also never leaves a frame empty, draws only characters every
/// supported font has, survives odd sizes and a resize to one cell and back. It also never
/// changes a large area more than three times a second.
pub fn check_scene(name: &str) {
    for &(width, height) in &SIZES {
        for motion in Motion::ALL {
            let every_frame = (width, height) == (80, 24) && motion == Motion::FULL;
            let steps = if every_frame { 60 * 20 } else { 30 * 20 };
            let mut rng = Rng::new(7);
            let mut scene = create(name, width, height, motion, &mut rng).expect(name);
            assert_eq!(scene.name(), name);
            let mut frame = Frame::new(width, height);
            for step in 0..steps {
                scene.step(0.05, &mut rng);
                if every_frame || step % 150 == 0 {
                    check_colours(scene.as_ref(), &mut frame, name, (width, height));
                }
            }
            // a resize to something tiny and back must never panic
            scene.resize(1, 1, &mut rng);
            scene.step(0.05, &mut rng);
            scene.draw(&mut Frame::new(1, 1), theme::default_theme());
            scene.resize(width, height, &mut rng);
            scene.step(0.05, &mut rng);
        }
    }
    for &(width, height) in &ODD_SIZES {
        let mut rng = Rng::new(3);
        let mut scene = create(name, width, height, Motion::LIVELY, &mut rng).expect(name);
        let mut frame = Frame::new(width, height);
        for _ in 0..40 {
            scene.step(0.05, &mut rng);
            scene.draw(&mut frame, theme::default_theme());
        }
    }
    check_flashes(name);
}

/// Runs a scene for a simulated minute at every motion setting and two sizes, measuring the
/// brightness of each region of every frame in the brightest and the darkest theme, and
/// fails if any second has more than three large changes.
pub fn check_flashes(name: &str) {
    let themes = [
        theme::by_id("deep-contrast-dark").expect("theme"),
        theme::by_id("deep-contrast-light").expect("theme"),
    ];
    for &(width, height) in &[(40, 12), (100, 30)] {
        for motion in Motion::ALL {
            let mut rng = Rng::new(11);
            let mut scene = create(name, width, height, motion, &mut rng).expect(name);
            let mut frame = Frame::new(width, height);
            let mut measured: Vec<Vec<Vec<f32>>> = vec![Vec::new(); themes.len()];
            for _ in 0..(60 * 20) {
                scene.step(0.05, &mut rng);
                for (theme, out) in themes.iter().zip(measured.iter_mut()) {
                    frame.clear();
                    scene.draw(&mut frame, theme);
                    out.push(regions(&frame, theme));
                }
            }
            for (theme, frames) in themes.iter().zip(&measured) {
                let most = most_changes_per_second(frames, 20);
                assert!(
                    most <= MAX_CHANGES_PER_SECOND,
                    "{name} at {width}x{height} with {motion:?} in {} has {most} large \
                     changes in one second",
                    theme.name
                );
            }
        }
    }
}

/// The most times any one cell changes within one second of `frames` drawn at `fps`.
pub fn max_changes_per_second(frames: &[Frame], fps: usize) -> usize {
    let Some(first) = frames.first() else {
        return 0;
    };
    let mut most = 0;
    for y in 0..first.height() as i32 {
        for x in 0..first.width() as i32 {
            let changes: Vec<usize> = (1..frames.len())
                .filter(|&i| frames[i].get(x, y) != frames[i - 1].get(x, y))
                .collect();
            let mut start = 0;
            for end in 0..changes.len() {
                while changes[end] - changes[start] >= fps {
                    start += 1;
                }
                most = most.max(end - start + 1);
            }
        }
    }
    most
}

/// The same seed draws the same frame, colours included, since a pixel scene's text alone is
/// blocks.
pub fn check_determinism(name: &str) {
    let draw = || {
        let mut rng = Rng::new(2024);
        let mut scene = create(name, 60, 20, Motion::FULL, &mut rng).expect(name);
        let mut frame = Frame::new(60, 20);
        for _ in 0..50 {
            scene.step(0.05, &mut rng);
        }
        scene.draw(&mut frame, theme::default_theme());
        frame
    };
    assert!(draw() == draw(), "{name} is not deterministic");
}

// the pixel checks

/// How far apart, in OKLab, a shape that carries meaning stays from each colour of the imagery
/// behind it.
pub const SHAPE_APART: f32 = 0.08;
/// How far apart two bands of imagery that mean different things stay, in OKLab.
pub const BAND_APART: f32 = 0.03;
/// The furthest any colour may land from its nearest entry in the 256 colour table, in OKLab.
pub const MAX_256_DRIFT: f32 = 0.16;
/// The fewest and the most colours one frame of a pixel scene may use: enough for real
/// gradients, few enough to keep the escape output small.
pub const COLOURS: std::ops::RangeInclusive<usize> = 12..=160;

/// Every pixel in a frame, the top then the bottom of each pixel cell.
pub fn pixels(frame: &Frame) -> Vec<Rgb> {
    frame
        .cells()
        .iter()
        .filter_map(|c| c.pixel_pair())
        .flat_map(|(a, b)| [a, b])
        .collect()
}

/// Fails unless two colours are at least `least` apart in OKLab in `theme`.
pub fn apart(theme: &ThemeData, a: Rgb, b: Rgb, least: f32, what: &str) {
    assert!(
        distance(a, b) >= least,
        "{what} collapse in {}: {} and {} are {:.3} apart",
        theme.name,
        a.hex(),
        b.hex(),
        distance(a, b)
    );
}

/// Fails unless every named colour is background imagery in `theme`: below the non-text
/// floor, so it never competes with text.
pub fn assert_imagery(theme: &ThemeData, colours: &[(&str, Rgb)]) {
    for (what, colour) in colours {
        assert!(
            theme.contrast_of(*colour) < IMAGERY_CEILING,
            "the {what} in {} is {:.2}:1, too strong for imagery",
            theme.name,
            theme.contrast_of(*colour)
        );
    }
}

/// Fails unless every named colour keeps the theme's floor, as anything carrying meaning
/// must, standing apart from each of the `behind` colours it is drawn over, in full colour
/// and in the 256 colour table.
pub fn assert_shapes(theme: &ThemeData, colours: &[(&str, Rgb)], behind: &[Rgb]) {
    let near = |c: Rgb| pixel_256(c, theme.background);
    for (what, colour) in colours {
        assert!(
            theme.contrast_of(*colour) >= theme.floor(),
            "the {what} in {} is {:.2}:1, below its {}:1 floor",
            theme.name,
            theme.contrast_of(*colour),
            theme.floor()
        );
        for back in behind {
            apart(theme, *colour, *back, SHAPE_APART, what);
            assert_ne!(
                near(*colour),
                near(*back),
                "the {what} merges with {} in 256 colours in {}",
                back.hex(),
                theme.name
            );
        }
    }
}

/// Fails unless each pair of colours stays apart, in full colour and in the 256 colour
/// table, so bands that mean different things never merge.
pub fn assert_apart(theme: &ThemeData, pairs: &[(&str, Rgb, Rgb)]) {
    let near = |c: Rgb| pixel_256(c, theme.background);
    for (what, a, b) in pairs {
        apart(theme, *a, *b, BAND_APART, what);
        assert_ne!(
            near(*a),
            near(*b),
            "{what} merge in 256 colours in {}",
            theme.name
        );
    }
}

/// A scene's colours in one theme, sorted into background imagery and shapes that carry
/// meaning.
pub struct Palette {
    pub imagery: Vec<(&'static str, Rgb)>,
    pub shapes: Vec<(&'static str, Rgb)>,
}

/// The palette a scene declares keeps its imagery under the ceiling and its shapes at the floor,
/// every shape apart from every colour of imagery in full colour and in the 256 colour table, in
/// every theme in the palette.
pub fn check_palette(palette: fn(&ThemeData) -> Palette) {
    for theme in theme::all() {
        let p = palette(theme);
        assert_imagery(theme, &p.imagery);
        let behind: Vec<Rgb> = p.imagery.iter().map(|(_, c)| *c).collect();
        assert_shapes(theme, &p.shapes, &behind);
    }
}

fn drawn(scene: &dyn Scene, size: (usize, usize), theme: &ThemeData) -> Frame {
    let mut frame = Frame::new(size.0, size.1);
    scene.draw(&mut frame, theme);
    frame
}

/// Cells that differ from one frame to the next, summed over twenty seconds (long enough for a
/// slow fail scene to get going, short enough that a scene which settles has not yet settled
/// at the standard pace), with the most times any one cell changes in a second.
fn cells_changed(name: &str, motion: Motion) -> (usize, usize) {
    let theme = theme::default_theme();
    let mut rng = Rng::new(9);
    let mut scene = create(name, 60, 20, motion, &mut rng).expect(name);
    let frames: Vec<Frame> = (0..(20 * 20))
        .map(|_| {
            scene.step(0.05, &mut rng);
            drawn(scene.as_ref(), (60, 20), theme)
        })
        .collect();
    let total = frames
        .windows(2)
        .map(|w| {
            w[0].cells()
                .iter()
                .zip(w[1].cells())
                .filter(|(a, b)| a != b)
                .count()
        })
        .sum();
    (total, max_changes_per_second(&frames, 20))
}

/// One still of a pixel scene in one theme, held to the imagery rules.
fn check_still(name: &str, theme: &ThemeData, frame: &Frame) {
    let label = format!("{name} in {}", theme.name);
    let cells = frame.width() * frame.height();
    let painted = frame.cells().iter().filter(|c| c.is_pixel()).count();
    assert!(
        painted * 10 >= cells * 8,
        "{label} paints only {painted} of {cells} cells"
    );
    let all = pixels(frame);
    let mut used = all.clone();
    used.sort_unstable();
    used.dedup();
    assert!(
        COLOURS.contains(&used.len()),
        "{label} uses {} colours",
        used.len()
    );
    // most of the picture is background imagery, so it never competes with text
    let backdrop: Vec<Rgb> = all
        .iter()
        .copied()
        .filter(|&p| theme.contrast_of(p) < IMAGERY_CEILING)
        .collect();
    assert!(backdrop.len() * 2 > all.len(), "{label} is mostly shapes");
    // with colour off the background imagery drops away and the shapes keep an outline
    let ink = frame
        .cells()
        .iter()
        .filter_map(|c| c.pixel_pair())
        .filter(|&(a, b)| plain_glyph(a, b, theme.background) != ' ')
        .count();
    let text = frame
        .cells()
        .iter()
        .filter(|c| !c.is_pixel() && c.ch != ' ')
        .count();
    // a scene whose shapes are characters, such as the code rain, has them as text instead
    assert!(
        (ink >= 12 || text >= 12) && ink * 10 <= painted * 6,
        "{label} keeps {ink} of {painted} cells with colour off"
    );
    let mut worst = 0.0_f32;
    for &colour in &used {
        worst = worst.max(distance(
            colour,
            xterm_colour(pixel_256(colour, theme.background)),
        ));
    }
    assert!(
        worst <= MAX_256_DRIFT,
        "{label} is {worst:.3} off in 256 colours"
    );
    // the backdrop's most common colour, which a shape must never melt into in 256 colours
    let mut counts: HashMap<Rgb, usize> = HashMap::new();
    for &p in &backdrop {
        *counts.entry(p).or_default() += 1;
    }
    let main = counts
        .iter()
        .max_by_key(|&(c, n)| (*n, *c))
        .map(|(c, _)| *c)
        .expect("backdrop");
    let near = |c: Rgb| pixel_256(c, theme.background);
    if let Some(shape) = all
        .iter()
        .find(|&&p| theme.contrast_of(p) >= IMAGERY_CEILING && near(p) == near(main))
    {
        panic!(
            "a shape melts into the backdrop of {label} in 256 colours: {} and {}",
            shape.hex(),
            main.hex()
        );
    }
}

/// The one check every scene painted on the pixel canvas gets on top of `check_scene`, in every
/// theme the palette holds: sizes from one cell to a very large screen and back, the flash
/// limits at every motion setting, a pause that holds the picture still and two stills held to
/// the imagery rules. The first still is `seconds` in, late enough for a fail scene to show what
/// has gone wrong; the second is seven seconds later. Text over the scene keeps the floor,
/// reduced motion makes fewer and slower changes and the same seed paints the same pixels.
pub fn check_pixel_scene(name: &str, seconds: f32) {
    let sizes = [
        (1, 1),
        (2, 2),
        (5, 3),
        (20, 8),
        (33, 9),
        (300, 90),
        (400, 120),
    ];
    let catalogue = Catalogue::built_in();
    for theme in theme::all() {
        let mut rng = Rng::new(6);
        let mut scene = create(name, 80, 24, Motion::LIVELY, &mut rng).expect(name);
        for &(w, h) in sizes.iter().chain(sizes.iter().rev()) {
            scene.resize(w, h, &mut rng);
            for _ in 0..20 {
                scene.step(0.05, &mut rng);
            }
            assert!(
                drawn(scene.as_ref(), (w, h), theme).drawn() > 0,
                "{name} at {w}x{h} in {}",
                theme.name
            );
        }

        for motion in Motion::ALL {
            let mut rng = Rng::new(12);
            let mut scene = create(name, 60, 20, motion, &mut rng).expect(name);
            let mut measured = Vec::new();
            for _ in 0..(30 * 20) {
                scene.step(0.05, &mut rng);
                measured.push(regions(&drawn(scene.as_ref(), (60, 20), theme), theme));
            }
            let most = most_changes_per_second(&measured, 20);
            assert!(
                most <= MAX_CHANGES_PER_SECOND,
                "{name} in {} with {motion:?}: {most} large changes in a second",
                theme.name
            );
        }

        let mut rng = Rng::new(5);
        let mut scene = create(name, 80, 24, Motion::FULL, &mut rng).expect(name);
        let mut done = 0;
        for look in [seconds, seconds + 7.0] {
            let until = (look * 20.0) as usize;
            while done < until {
                scene.step(0.05, &mut rng);
                done += 1;
            }
            let frame = drawn(scene.as_ref(), (80, 24), theme);
            // a paused scene is drawn again without a step, so it must not move at all
            assert!(
                frame == drawn(scene.as_ref(), (80, 24), theme),
                "{name} moves while paused in {}",
                theme.name
            );
            check_still(name, theme, &frame);
        }

        for state in [SnapshotState::Pass, SnapshotState::Fail] {
            let shot = Shot {
                catalogue: &catalogue,
                scene: name,
                theme,
                size: (80, 24),
                seed: 1,
                motion: Motion::FULL,
                state: Some(state),
                clock: crate::clock::SNAPSHOT_START,
            };
            let frame = shot.still(3.0).expect(name);
            let mut text = 0;
            for cell in frame
                .cells()
                .iter()
                .filter(|c| !c.is_pixel() && c.ch != ' ')
            {
                text += 1;
                let fg = cell.colour.unwrap_or(theme.foreground);
                let bg = cell.bg.unwrap_or(theme.background);
                assert!(
                    fg.contrast(bg) >= theme.floor(),
                    "{:?} over {name} in {} is {:.2}:1",
                    cell.ch,
                    theme.name,
                    fg.contrast(bg)
                );
            }
            assert!(text > 10, "no banner over {name} in {}", theme.name);
        }
    }

    let (full, full_most) = cells_changed(name, Motion::FULL);
    let (calm, calm_most) = cells_changed(name, Motion::CALM);
    assert!(
        calm * 4 < full * 3,
        "{name}: {calm} cells changed calm, {full} full"
    );
    assert!(
        calm_most <= full_most,
        "{name}: a cell changes {calm_most} times a second calm, {full_most} full"
    );

    // the same seed paints exactly the same pixels, not only the same characters
    let paint = || {
        let mut rng = Rng::new(2024);
        let mut scene = create(name, 70, 22, Motion::FULL, &mut rng).expect(name);
        for _ in 0..90 {
            scene.step(0.05, &mut rng);
        }
        drawn(scene.as_ref(), (70, 22), theme::default_theme())
    };
    assert!(paint() == paint(), "{name} is not deterministic");
}

/// The shortest gap between two changes of `on`, sampled at 20 frames a second for
/// `seconds`.
pub fn shortest_gap(seconds: f32, mut on: impl FnMut(f64) -> u32) -> f32 {
    let mut last_change: Option<f32> = None;
    let mut shortest = f32::MAX;
    let mut previous = on(0.0);
    for i in 1..(seconds * 20.0) as usize {
        let t = i as f32 * 0.05;
        let now = on(f64::from(t));
        if now != previous {
            if let Some(at) = last_change {
                shortest = shortest.min(t - at);
            }
            last_change = Some(t);
            previous = now;
        }
    }
    shortest
}
