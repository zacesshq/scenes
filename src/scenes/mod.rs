//! Every built-in scene and the registry the command line, the watch mode and the config pick
//! from.

pub mod aurora;
pub mod autumn;
pub mod balloons;
pub mod blackout;
pub mod blossom;
pub mod broken_bridge;
pub mod campfire;
pub mod cat_nap;
pub mod city_night;
pub mod code_rain;
pub mod constellations;
pub mod corrosion;
pub mod countryside;
pub mod drought;
pub mod erosion;
pub mod fireflies;
pub mod flood;
pub mod fog;
pub mod garden;
pub mod harbour;
pub mod koi_pond;
pub mod lanterns;
pub mod leak;
pub mod listing_ship;
pub mod orbit;
pub mod rain;
pub mod ribbons;
pub mod sea;
pub mod snow;
pub mod starfield;
pub mod tangle;
pub mod tea;
pub mod tiles;
pub mod traffic_jam;
pub mod waves;
pub mod wilt;
pub mod winter_cabin;
pub mod zacess;

use crate::rng::Rng;
use crate::scene::{Motion, Scene};

/// What a scene is for. Pass and general scenes can be shown while a build passes, fail scenes
/// while it fails. Every scene runs on its own too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A calm scene made for a passing build.
    Pass,
    /// A slow, serious scene that reads as something gone wrong, made for a failing build.
    Fail,
    /// One of the original scenes, calm enough to show while a build passes.
    General,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Pass => "pass",
            Kind::Fail => "fail",
            Kind::General => "general",
        }
    }

    pub fn parse(text: &str) -> Option<Kind> {
        match text.trim().to_ascii_lowercase().as_str() {
            "pass" => Some(Kind::Pass),
            "fail" => Some(Kind::Fail),
            "general" => Some(Kind::General),
            _ => None,
        }
    }

    /// Whether a scene of this kind can stand for a passing build.
    pub fn passes(self) -> bool {
        self != Kind::Fail
    }
}

/// A scene's name, one line about it, its kind and the captions the watch mode shows under its
/// banner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub kind: Kind,
    pub captions: &'static [&'static str],
}

const fn general(
    name: &'static str,
    summary: &'static str,
    captions: &'static [&'static str],
) -> SceneInfo {
    SceneInfo {
        name,
        summary,
        kind: Kind::General,
        captions,
    }
}

/// Every scene, in the order the help lists them: the original ten, then the pass scenes, then
/// the fail scenes.
pub const SCENES: [SceneInfo; 39] = [
    general(
        "sea",
        "Fish of several sizes, bubbles, seaweed, a boat on the surface and an occasional whale",
        &[
            "Clean build. The fish approve.",
            "Not a ripple out of place.",
            "Everything compiles. Swim on.",
        ],
    ),
    general(
        "rain",
        "Drops and puddle ripples, with distant lightning unless motion is reduced",
        &[
            "A soft rain and a clean build.",
            "Nothing to fix. Listen to the rain.",
            "All green under grey skies.",
        ],
    ),
    general(
        "snow",
        "Drifting flakes that settle at the bottom",
        &[
            "Quiet as fresh snow.",
            "A clean run. Settle in.",
            "Not a single error in the drift.",
        ],
    ),
    general(
        "starfield",
        "Twinkling stars, a crescent moon and a slow shooting star",
        &[
            "The build passed. Make a wish anyway.",
            "Clear skies over the compiler.",
            "Every star in its place.",
        ],
    ),
    general(
        "code-rain",
        "Falling columns of characters in the theme's syntax colours",
        &[
            "Every column compiles.",
            "The code falls into place.",
            "No errors in the stream.",
        ],
    ),
    general(
        "fireflies",
        "A dark meadow with slow pulsing lights",
        &[
            "A quiet meadow and a clean build.",
            "Small lights, no warnings.",
            "Nothing to chase tonight.",
        ],
    ),
    general(
        "aurora",
        "Slow waves of colour across the top of the sky",
        &[
            "The sky approves of this build.",
            "Green from end to end and then some.",
            "A calm night over a clean build.",
        ],
    ),
    general(
        "campfire",
        "Flames and embers with a steady glow",
        &[
            "Pull up a log. The build is fine.",
            "Warm and green.",
            "Nothing left to tend but the fire.",
        ],
    ),
    general(
        "waves",
        "A calm sea horizon with gulls",
        &[
            "Everything builds. Time to stretch your legs.",
            "A clean run. The borrow checker is content.",
            "Nothing to fix. On to the next thing.",
            "All quiet in the compiler.",
            "Green from end to end.",
            "The build is happy. So is the sea.",
        ],
    ),
    general(
        "zacess",
        "The Zacess mark drifting slowly",
        &[
            "Clean, clear and readable.",
            "A tidy build.",
            "All good on our side.",
        ],
    ),
    SceneInfo {
        name: "koi-pond",
        summary: koi_pond::SUMMARY,
        kind: Kind::Pass,
        captions: koi_pond::CAPTIONS,
    },
    SceneInfo {
        name: "harbour",
        summary: harbour::SUMMARY,
        kind: Kind::Pass,
        captions: harbour::CAPTIONS,
    },
    SceneInfo {
        name: "lanterns",
        summary: lanterns::SUMMARY,
        kind: Kind::Pass,
        captions: lanterns::CAPTIONS,
    },
    SceneInfo {
        name: "balloons",
        summary: balloons::SUMMARY,
        kind: Kind::Pass,
        captions: balloons::CAPTIONS,
    },
    SceneInfo {
        name: "countryside",
        summary: countryside::SUMMARY,
        kind: Kind::Pass,
        captions: countryside::CAPTIONS,
    },
    SceneInfo {
        name: "winter-cabin",
        summary: winter_cabin::SUMMARY,
        kind: Kind::Pass,
        captions: winter_cabin::CAPTIONS,
    },
    SceneInfo {
        name: "autumn",
        summary: autumn::SUMMARY,
        kind: Kind::Pass,
        captions: autumn::CAPTIONS,
    },
    SceneInfo {
        name: "blossom",
        summary: blossom::SUMMARY,
        kind: Kind::Pass,
        captions: blossom::CAPTIONS,
    },
    SceneInfo {
        name: "city-night",
        summary: city_night::SUMMARY,
        kind: Kind::Pass,
        captions: city_night::CAPTIONS,
    },
    SceneInfo {
        name: "orbit",
        summary: orbit::SUMMARY,
        kind: Kind::Pass,
        captions: orbit::CAPTIONS,
    },
    SceneInfo {
        name: "constellations",
        summary: constellations::SUMMARY,
        kind: Kind::Pass,
        captions: constellations::CAPTIONS,
    },
    SceneInfo {
        name: "tea",
        summary: tea::SUMMARY,
        kind: Kind::Pass,
        captions: tea::CAPTIONS,
    },
    SceneInfo {
        name: "cat-nap",
        summary: cat_nap::SUMMARY,
        kind: Kind::Pass,
        captions: cat_nap::CAPTIONS,
    },
    SceneInfo {
        name: "garden",
        summary: garden::SUMMARY,
        kind: Kind::Pass,
        captions: garden::CAPTIONS,
    },
    SceneInfo {
        name: "tiles",
        summary: tiles::SUMMARY,
        kind: Kind::Pass,
        captions: tiles::CAPTIONS,
    },
    SceneInfo {
        name: "ribbons",
        summary: ribbons::SUMMARY,
        kind: Kind::Pass,
        captions: ribbons::CAPTIONS,
    },
    SceneInfo {
        name: "storm",
        summary: "Heavy slanting rain under two rows of cloud, with distant lightning unless motion is reduced",
        kind: Kind::Fail,
        captions: &[
            "Something broke. The storm will pass.",
            "Rough weather in the build.",
            "Batten down and read the first error.",
        ],
    },
    SceneInfo {
        name: "flood",
        summary: flood::SUMMARY,
        kind: Kind::Fail,
        captions: flood::CAPTIONS,
    },
    SceneInfo {
        name: "listing-ship",
        summary: listing_ship::SUMMARY,
        kind: Kind::Fail,
        captions: listing_ship::CAPTIONS,
    },
    SceneInfo {
        name: "corrosion",
        summary: corrosion::SUMMARY,
        kind: Kind::Fail,
        captions: corrosion::CAPTIONS,
    },
    SceneInfo {
        name: "tangle",
        summary: tangle::SUMMARY,
        kind: Kind::Fail,
        captions: tangle::CAPTIONS,
    },
    SceneInfo {
        name: "wilt",
        summary: wilt::SUMMARY,
        kind: Kind::Fail,
        captions: wilt::CAPTIONS,
    },
    SceneInfo {
        name: "fog",
        summary: fog::SUMMARY,
        kind: Kind::Fail,
        captions: fog::CAPTIONS,
    },
    SceneInfo {
        name: "broken-bridge",
        summary: broken_bridge::SUMMARY,
        kind: Kind::Fail,
        captions: broken_bridge::CAPTIONS,
    },
    SceneInfo {
        name: "leak",
        summary: leak::SUMMARY,
        kind: Kind::Fail,
        captions: leak::CAPTIONS,
    },
    SceneInfo {
        name: "blackout",
        summary: blackout::SUMMARY,
        kind: Kind::Fail,
        captions: blackout::CAPTIONS,
    },
    SceneInfo {
        name: "traffic-jam",
        summary: traffic_jam::SUMMARY,
        kind: Kind::Fail,
        captions: traffic_jam::CAPTIONS,
    },
    SceneInfo {
        name: "erosion",
        summary: erosion::SUMMARY,
        kind: Kind::Fail,
        captions: erosion::CAPTIONS,
    },
    SceneInfo {
        name: "drought",
        summary: drought::SUMMARY,
        kind: Kind::Fail,
        captions: drought::CAPTIONS,
    },
];

pub fn info(name: &str) -> Option<&'static SceneInfo> {
    SCENES.iter().find(|s| s.name == name)
}

/// The names of every scene of a kind, in the help's order.
pub fn names_of(kinds: &[Kind]) -> Vec<&'static str> {
    SCENES
        .iter()
        .filter(|s| kinds.contains(&s.kind))
        .map(|s| s.name)
        .collect()
}

/// Builds a scene by name, fitted to a size.
pub fn create(
    name: &str,
    width: usize,
    height: usize,
    motion: Motion,
    rng: &mut Rng,
) -> Option<Box<dyn Scene>> {
    let scene: Box<dyn Scene> = match name {
        "sea" => Box::new(sea::Sea::new(width, height, motion, rng)),
        "rain" => Box::new(rain::Rain::new(width, height, motion, rng)),
        "storm" => Box::new(rain::Rain::storm(width, height, motion, rng)),
        "snow" => Box::new(snow::Snow::new(width, height, motion, rng)),
        "starfield" => Box::new(starfield::Starfield::new(width, height, motion, rng)),
        "code-rain" => Box::new(code_rain::CodeRain::new(width, height, motion, rng)),
        "fireflies" => Box::new(fireflies::Fireflies::new(width, height, motion, rng)),
        "aurora" => Box::new(aurora::Aurora::new(width, height, motion, rng)),
        "campfire" => Box::new(campfire::Campfire::new(width, height, motion, rng)),
        "waves" => Box::new(waves::Waves::new(width, height, motion, rng)),
        "zacess" => Box::new(zacess::Zacess::new(width, height, motion, rng)),
        "koi-pond" => Box::new(koi_pond::KoiPond::new(width, height, motion, rng)),
        "harbour" => Box::new(harbour::Harbour::new(width, height, motion, rng)),
        "lanterns" => Box::new(lanterns::Lanterns::new(width, height, motion, rng)),
        "balloons" => Box::new(balloons::Balloons::new(width, height, motion, rng)),
        "countryside" => Box::new(countryside::Countryside::new(width, height, motion, rng)),
        "winter-cabin" => Box::new(winter_cabin::WinterCabin::new(width, height, motion, rng)),
        "autumn" => Box::new(autumn::Autumn::new(width, height, motion, rng)),
        "blossom" => Box::new(blossom::Blossom::new(width, height, motion, rng)),
        "city-night" => Box::new(city_night::CityNight::new(width, height, motion, rng)),
        "orbit" => Box::new(orbit::Orbit::new(width, height, motion, rng)),
        "constellations" => Box::new(constellations::Constellations::new(
            width, height, motion, rng,
        )),
        "tea" => Box::new(tea::Tea::new(width, height, motion, rng)),
        "cat-nap" => Box::new(cat_nap::CatNap::new(width, height, motion, rng)),
        "garden" => Box::new(garden::Garden::new(width, height, motion, rng)),
        "tiles" => Box::new(tiles::Tiles::new(width, height, motion, rng)),
        "ribbons" => Box::new(ribbons::Ribbons::new(width, height, motion, rng)),
        "flood" => Box::new(flood::Flood::new(width, height, motion, rng)),
        "listing-ship" => Box::new(listing_ship::ListingShip::new(width, height, motion, rng)),
        "corrosion" => Box::new(corrosion::Corrosion::new(width, height, motion, rng)),
        "tangle" => Box::new(tangle::Tangle::new(width, height, motion, rng)),
        "wilt" => Box::new(wilt::Wilt::new(width, height, motion, rng)),
        "fog" => Box::new(fog::Fog::new(width, height, motion, rng)),
        "broken-bridge" => Box::new(broken_bridge::BrokenBridge::new(width, height, motion, rng)),
        "leak" => Box::new(leak::Leak::new(width, height, motion, rng)),
        "blackout" => Box::new(blackout::Blackout::new(width, height, motion, rng)),
        "traffic-jam" => Box::new(traffic_jam::TrafficJam::new(width, height, motion, rng)),
        "erosion" => Box::new(erosion::Erosion::new(width, height, motion, rng)),
        "drought" => Box::new(drought::Drought::new(width, height, motion, rng)),
        _ => return None,
    };
    Some(scene)
}

/// How many of something a screen gets: one per `cells_per_item` cells, within a range, so a
/// scene is neither empty on a phone nor sparse on a wide display.
pub(crate) fn count_for(
    width: usize,
    height: usize,
    cells_per_item: usize,
    min: usize,
    max: usize,
) -> usize {
    ((width * height) / cells_per_item.max(1)).clamp(min, max)
}

/// The angle `rate * t` in radians, wrapped to one turn before it becomes an `f32`. Scene time
/// is an `f64` and is only narrowed here, so a scene left running for weeks still moves
/// smoothly instead of stepping or freezing as an `f32` clock would.
pub(crate) fn cycle(t: f64, rate: f32) -> f32 {
    (t * f64::from(rate)).rem_euclid(std::f64::consts::TAU) as f32
}

/// Wraps a position into `[0, size)`, for things that leave one edge and return at the other.
pub(crate) fn wrap(value: f32, size: f32) -> f32 {
    if size <= 0.0 {
        return 0.0;
    }
    let v = value % size;
    if v < 0.0 { v + size } else { v }
}

#[cfg(test)]
pub(crate) mod testing {
    //! Checks shared by every scene's tests.

    use super::*;
    use crate::frame::Frame;
    use crate::theme;

    /// The sizes every scene must look right at: a phone-sized terminal, a classic terminal, a
    /// full screen and a very wide display.
    pub const SIZES: [(usize, usize); 4] = [(20, 8), (80, 24), (120, 40), (200, 60)];

    /// Odd sizes a scene must survive without a panic, smaller than it is meant for included.
    pub const ODD_SIZES: [(usize, usize); 6] =
        [(1, 1), (2, 2), (7, 3), (19, 7), (33, 9), (300, 90)];

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
        use crate::flash::{MAX_CHANGES_PER_SECOND, most_changes_per_second, regions};
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

    /// The same seed draws the same frame.
    pub fn check_determinism(name: &str) {
        let mut frames = Vec::new();
        for _ in 0..2 {
            let mut rng = Rng::new(2024);
            let mut scene = create(name, 60, 20, Motion::FULL, &mut rng).expect(name);
            let mut frame = Frame::new(60, 20);
            for _ in 0..50 {
                scene.step(0.05, &mut rng);
            }
            scene.draw(&mut frame, theme::default_theme());
            frames.push(frame.to_text());
        }
        assert_eq!(frames[0], frames[1], "{name} is not deterministic");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_scene_can_be_built() {
        let mut rng = Rng::new(1);
        for info in &SCENES {
            let scene = create(info.name, 40, 12, Motion::FULL, &mut rng);
            assert!(scene.is_some(), "{}", info.name);
            assert_eq!(scene.unwrap().name(), info.name);
        }
        assert!(create("lava", 40, 12, Motion::FULL, &mut rng).is_none());
        let mut names: Vec<&str> = SCENES.iter().map(|s| s.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SCENES.len(), "two scenes share a name");
        assert_eq!(info("snow").map(|i| i.name), Some("snow"));
    }

    #[test]
    fn there_are_more_than_25_pass_scenes_and_12_fail_scenes_each_with_captions() {
        let pass = SCENES.iter().filter(|s| s.kind.passes()).count();
        let fail = SCENES.iter().filter(|s| s.kind == Kind::Fail).count();
        assert!(pass > 25, "{pass}");
        assert!(fail > 12, "{fail}");
        for scene in &SCENES {
            assert!(scene.captions.len() >= 3, "{} needs captions", scene.name);
            assert!(!scene.summary.is_empty());
            for caption in scene.captions {
                assert!(caption.chars().count() <= 60, "{caption}");
                assert!(!caption.contains('\u{2014}') && !caption.contains('\u{2013}'));
            }
        }
        assert_eq!(Kind::parse("FAIL"), Some(Kind::Fail));
        assert_eq!(names_of(&[Kind::Fail]).len(), fail);
    }

    #[test]
    fn counts_scale_with_area_within_bounds() {
        assert_eq!(count_for(20, 8, 10, 3, 50), 16);
        assert_eq!(count_for(20, 8, 1000, 3, 50), 3);
        assert_eq!(count_for(200, 60, 10, 3, 50), 50);
        assert_eq!(count_for(10, 10, 0, 1, 5), 5);
    }

    #[test]
    fn wrap_keeps_values_in_range() {
        assert_eq!(wrap(-1.0, 10.0), 9.0);
        assert_eq!(wrap(10.0, 10.0), 0.0);
        assert_eq!(wrap(3.5, 10.0), 3.5);
        assert_eq!(wrap(5.0, 0.0), 0.0);
    }
}
