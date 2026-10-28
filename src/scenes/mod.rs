//! The ten scenes and the registry the command line picks from.

pub mod aurora;
pub mod campfire;
pub mod code_rain;
pub mod fireflies;
pub mod rain;
pub mod sea;
pub mod snow;
pub mod starfield;
pub mod waves;
pub mod zacess;

use crate::rng::Rng;
use crate::scene::{Motion, Scene};

/// A scene's name and one line about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneInfo {
    pub name: &'static str,
    pub summary: &'static str,
}

/// Every scene, in the order the help lists them.
pub const SCENES: [SceneInfo; 10] = [
    SceneInfo {
        name: "sea",
        summary: "Fish of several sizes, bubbles, seaweed, a boat on the surface and an occasional whale",
    },
    SceneInfo {
        name: "rain",
        summary: "Drops and puddle ripples, with distant lightning unless motion is reduced",
    },
    SceneInfo {
        name: "snow",
        summary: "Drifting flakes that settle at the bottom",
    },
    SceneInfo {
        name: "starfield",
        summary: "Twinkling stars, a crescent moon and a slow shooting star",
    },
    SceneInfo {
        name: "code-rain",
        summary: "Falling columns of characters in the theme's syntax colours",
    },
    SceneInfo {
        name: "fireflies",
        summary: "A dark meadow with slow pulsing lights",
    },
    SceneInfo {
        name: "aurora",
        summary: "Slow waves of colour across the top of the sky",
    },
    SceneInfo {
        name: "campfire",
        summary: "Flames and embers with a steady glow",
    },
    SceneInfo {
        name: "waves",
        summary: "A calm sea horizon with gulls",
    },
    SceneInfo {
        name: "zacess",
        summary: "The Zacess mark drifting slowly",
    },
];

pub fn info(name: &str) -> Option<&'static SceneInfo> {
    SCENES.iter().find(|s| s.name == name)
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
        "snow" => Box::new(snow::Snow::new(width, height, motion, rng)),
        "starfield" => Box::new(starfield::Starfield::new(width, height, motion, rng)),
        "code-rain" => Box::new(code_rain::CodeRain::new(width, height, motion, rng)),
        "fireflies" => Box::new(fireflies::Fireflies::new(width, height, motion, rng)),
        "aurora" => Box::new(aurora::Aurora::new(width, height, motion, rng)),
        "campfire" => Box::new(campfire::Campfire::new(width, height, motion, rng)),
        "waves" => Box::new(waves::Waves::new(width, height, motion, rng)),
        "zacess" => Box::new(zacess::Zacess::new(width, height, motion, rng)),
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

    /// The sizes every scene must look right at: a phone-sized terminal and a full screen.
    pub const SIZES: [(usize, usize); 3] = [(20, 8), (80, 24), (120, 40)];

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

    /// Checks every colour in one drawn frame against each theme's floor.
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
    }

    /// Every colour a scene draws, in every theme, keeps that theme's contrast floor on its
    /// background: every frame of a minute at 80 by 24, long enough for lightning, shooting stars
    /// and the whale, plus sampled frames at a phone size and a full screen. The scene also never
    /// leaves a frame empty and survives a resize to one cell and back.
    pub fn check_scene(name: &str) {
        for &(width, height) in &SIZES {
            let every_frame = (width, height) == (80, 24);
            let steps = if every_frame { 60 * 20 } else { 120 };
            for motion in [Motion::FULL, Motion::CALM] {
                let mut rng = Rng::new(7);
                let mut scene = create(name, width, height, motion, &mut rng).expect(name);
                assert_eq!(scene.name(), name);
                let mut frame = Frame::new(width, height);
                for step in 0..steps {
                    scene.step(0.05, &mut rng);
                    if every_frame || step % 40 == 0 {
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
        assert_eq!(info("snow").map(|i| i.name), Some("snow"));
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
