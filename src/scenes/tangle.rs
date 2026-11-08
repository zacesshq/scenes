//! Tangle: a patch panel across the top of a dim server room wall with round cables hanging from
//! its ports. They are already crossed when the scene opens and keep wandering: across each
//! other, into loops and knots, down to the floor where their ends trail off in loose curves,
//! while the snarl sways slowly. One port's link light has gone red.
//!
//! The wall and the floor are tints of the background held below the non-text floor. The panel
//! and the cables are solid, each cable drawn as a tube lit from the upper left with a dark rim,
//! so where one crosses another the nearer one reads on top and with colour off the snarl keeps
//! its shape.

use std::f32::consts::{PI, TAU};

use super::paint::{ease, shaded, solid, stronger};
use super::{count_for, cycle};
use crate::colour::Rgb;
use crate::frame::Frame;
use crate::pixels::{Canvas, Ramp, Shades, mix};
use crate::rng::Rng;
use crate::scene::{Motion, Scene};
use crate::theme::ThemeData;
use crate::watch::view::error_colour;

/// Seconds the cables take to go from crossed to fully knotted at the standard pace.
pub const TANGLE_SECONDS: f32 = 45.0;
/// How tangled the cables already are when the scene opens, so it reads as a snarl at once.
const START: f32 = 0.45;
/// Radians a second the snarl sways by. It moves the cables a fraction of a pixel a second, so a
/// pixel changes as a cable drifts across it, never by flicking back and forth.
const SWAY_RATE: f32 = 0.35;

pub const SUMMARY: &str =
    "Round cables hanging from a patch panel that slowly wander, cross and knot into a snarl";

pub const CAPTIONS: &[&str] = &[
    "A few crossed wires. Trace the first one back.",
    "Untangle one cable at a time.",
    "Follow the error to the port it came from.",
    "Knotted, not cut. It comes apart with patience.",
    "Labels help. So does the first error.",
];

#[derive(Clone, Debug)]
struct Cable {
    port: f32,
    /// Where the cable drifts to on its way down once fully tangled, as an offset from the port.
    reach: f32,
    amplitude: f32,
    waves: f32,
    phase: f32,
    colour: usize,
    /// Where along the cable a loop forms, its size and which way it turns.
    knot: Option<(f32, f32, f32)>,
    /// Which way the end trails along the floor and how far.
    trail: f32,
}

/// One cable's tones: the dark rim, the shaded side, the body, the lit side and its sheen.
type Tube = [Rgb; 5];

/// The room's colours in one theme.
struct Colours {
    wall: Ramp,
    floor: Ramp,
    seam: Rgb,
    tile: Rgb,
    panel: (Rgb, Rgb, Rgb),
    socket: Rgb,
    screw: Rgb,
    ok: Rgb,
    fault: Rgb,
    cables: [Tube; 6],
}

impl Colours {
    fn new(theme: &ThemeData) -> Self {
        let shades = Shades::new(theme);
        let (wall_top, wall_low, floor_top, floor_low) = if theme.dark {
            (
                shades.tint(theme.info, 0.08, 1.15),
                shades.tint(theme.info, 0.22, 1.45),
                shades.tint(theme.muted, 0.34, 1.85),
                shades.tint(theme.muted, 0.2, 1.4),
            )
        } else {
            (
                shades.tint(theme.info, 0.12, 1.2),
                shades.tint(theme.muted, 0.3, 1.6),
                shades.tint(theme.muted, 0.5, 2.15),
                shades.tint(theme.muted, 0.68, 2.6),
            )
        };
        let panel = solid(theme, theme.muted, shades.darkest(), 0.4);
        let tube = |c: Rgb| -> Tube {
            let (dark, base, light) = shaded(theme, c);
            // the rim sits between the cable and the room: a deep shade of the cable, held
            // under the ceiling so it reads as the edge of the tube rather than more cable
            let rim = shades.tint(dark, 0.8, 2.6);
            [rim, dark, base, light, stronger(theme, light, 0.5)]
        };
        Self {
            wall: Ramp::new(wall_top, wall_low, 8),
            floor: Ramp::new(floor_top, floor_low, 4),
            seam: mix(wall_top, theme.background, 0.5),
            tile: mix(floor_top, floor_low, 0.5),
            panel: shaded(theme, panel),
            socket: mix(panel, shades.darkest(), 0.75),
            screw: stronger(theme, panel, 0.6),
            ok: theme.green(),
            fault: error_colour(theme),
            cables: [
                tube(theme.blue()),
                tube(theme.yellow()),
                tube(theme.magenta()),
                tube(theme.green()),
                tube(theme.cyan()),
                tube(theme.red()),
            ],
        }
    }
}

pub struct Tangle {
    width: usize,
    height: usize,
    motion: Motion,
    t: f64,
    /// Seconds of tangling so far, kept across a resize so the cables never straighten.
    tangled: f32,
    cables: Vec<Cable>,
    /// The cable whose port shows the fault light.
    faulty: usize,
}

impl Tangle {
    pub fn new(width: usize, height: usize, motion: Motion, rng: &mut Rng) -> Self {
        let mut tangle = Self {
            width: 1,
            height: 1,
            motion,
            t: 0.0,
            tangled: 0.0,
            cables: Vec::new(),
            faulty: 0,
        };
        tangle.resize(width, height, rng);
        tangle
    }

    fn tall(&self) -> i32 {
        self.height as i32 * 2
    }

    /// The pixel rows of the patch panel.
    fn panel_rows(&self) -> i32 {
        ((self.tall() as f32 * 0.13) as i32)
            .clamp(5, 14)
            .min(self.tall() - 1)
    }

    /// The top pixel row the cables hang from, under the panel.
    fn top(&self) -> i32 {
        self.panel_rows()
    }

    /// The first pixel row of the floor.
    fn floor(&self) -> i32 {
        (self.tall() - (self.tall() / 7).max(2)).max(self.top() + 1)
    }

    /// The pixel row the cables come to rest on, lying on the floor.
    fn bottom(&self) -> i32 {
        (self.floor() + (self.tall() - self.floor()) / 2).max(self.top())
    }

    /// Half the thickness of a cable's body in pixels.
    fn radius(&self) -> f32 {
        (self.tall() as f32 / 52.0).clamp(1.0, 2.8)
    }

    fn amount(&self) -> f32 {
        START + (1.0 - START) * ease(self.tangled / TANGLE_SECONDS)
    }

    /// The point a cable passes through `s` of the way along it, from its port at 0 to its end
    /// trailing on the floor at 1.
    fn point(&self, cable: &Cable, s: f32) -> (f32, f32) {
        let (top, bottom) = (self.top() as f32, self.bottom() as f32);
        let span = (bottom - top).max(1.0);
        let amount = self.amount();
        let sway = cycle(self.t, SWAY_RATE * self.motion.speed());
        // the cable falls for most of its length, then curves down onto the floor
        let fall = (s / 0.9).min(1.0);
        let mut y = top + span * (1.0 - (1.0 - s).powf(2.2)).max(fall * 0.8);
        let drift = cable.reach * amount * ease(fall);
        let wave = cable.amplitude
            * amount
            * (PI * fall).sin()
            * (fall * PI * cable.waves + cable.phase + sway).sin();
        let mut x = cable.port + drift + wave;
        if let Some((at, size, turn)) = cable.knot {
            // a loop: around the knot the cable swings out in a circle and back up over itself
            let grown = ((amount - START) / (1.0 - START) * 1.5 + 0.4).min(1.0);
            let width = 0.16;
            let u = ((fall - at) / width).clamp(0.0, 1.0);
            if u > 0.0 && u < 1.0 {
                let a = u * TAU;
                let r = size * grown;
                x += turn * r * a.sin();
                y -= r * (1.0 - a.cos()) * 1.1;
            }
        }
        if s > 0.55 {
            // the end trails off along the floor in an easy curve
            let along = (s - 0.55) / 0.45;
            x += cable.trail * amount * along * along;
        }
        (x, y.min(bottom))
    }

    fn paint(&self, canvas: &mut Canvas, c: &Colours) {
        let wide = canvas.width() as i32;
        let tall = canvas.height() as i32;
        let floor = self.floor();
        let panel_rows = self.panel_rows();
        // the wall: a soft fall of light from the panel lamp above, with tall panel seams
        for y in 0..floor {
            canvas.fill_row(y, c.wall.at(y as f32 / floor.max(1) as f32));
        }
        let seam_gap = ((tall as f32 * 0.4) as i32).max(12);
        for x in (seam_gap / 2..wide).step_by(seam_gap as usize) {
            for y in panel_rows..floor {
                canvas.set(x, y, c.seam);
            }
        }
        // a raised floor of tiles, its joins running back to the wall
        for y in floor..tall {
            let v = (y - floor) as f32 / (tall - floor).max(1) as f32;
            canvas.fill_row(y, c.floor.at(v));
        }
        canvas.fill_row(floor, c.seam);
        let tile = ((tall as f32 * 0.35) as i32).max(10);
        for k in (-wide / tile - 1)..(wide / tile + 2) {
            let base = wide / 2 + k * tile;
            for y in floor + 1..tall {
                let spread = (y - floor) as f32 / (tall - floor).max(1) as f32;
                let x = base as f32 + (base - wide / 2) as f32 * spread * 0.4;
                canvas.set(x.round() as i32, y, c.tile);
            }
        }

        self.paint_panel(canvas, c, wide);

        let r = self.radius();
        for cable in &self.cables {
            let tube = c.cables[cable.colour % c.cables.len()];
            // the plug, square in its socket, then the cable
            let port = cable.port.round() as i32;
            let plug = (r.round() as i32).max(1);
            canvas.fill_rect(port - plug, panel_rows - 4, plug * 2 + 1, 4, tube[2]);
            canvas.fill_rect(port - plug, panel_rows - 4, 1, 4, tube[3]);
            canvas.fill_rect(port + plug, panel_rows - 4, 1, 4, tube[1]);
            self.paint_cable(canvas, cable, tube, r);
        }
    }

    fn paint_panel(&self, canvas: &mut Canvas, c: &Colours, wide: i32) {
        let rows = self.panel_rows();
        let (dark, body, light) = c.panel;
        canvas.fill_rect(0, 0, wide, rows - 1, body);
        canvas.fill_row(0, light);
        canvas.fill_row(rows - 1, dark);
        // the screws that hold it in the rack, at each end
        for x in [2, wide - 3] {
            canvas.set(x, 2, c.screw);
            canvas.set(x, rows - 3, c.screw);
        }
        // a socket for every port, the empty ones between the cables as well, each with its link
        // light; only the faulty cable's port is red
        let pitch = self.port_pitch();
        let mut x = 6.0;
        while x < wide as f32 - 6.0 {
            let px = x.round() as i32;
            canvas.fill_rect(px - 1, rows - 5, 3, 3, c.socket);
            x += pitch;
        }
        for (i, cable) in self.cables.iter().enumerate() {
            let px = cable.port.round() as i32;
            let plug = (self.radius().round() as i32).max(1);
            canvas.fill_rect(px - plug - 1, rows - 5, plug * 2 + 3, 4, c.socket);
            let light = if i == self.faulty { c.fault } else { c.ok };
            canvas.fill_rect(px - 1, 1, 2, 2.min(rows - 6).max(1), light);
        }
    }

    fn port_pitch(&self) -> f32 {
        let gap = self.width as f32 / self.cables.len().max(1) as f32;
        (gap / 2.0).max(4.0)
    }

    /// Draws a cable as a round tube. Each pixel within reach of the cable's centre line takes
    /// its tone from how far it lies from that line and on which side, so the body is shaded
    /// smoothly across its width, lit on the side facing the upper left, with a rim at its edge.
    fn paint_cable(&self, canvas: &mut Canvas, cable: &Cable, tube: Tube, r: f32) {
        let span = (self.bottom() - self.top()).max(1) as f32 * 2.0 + cable.trail.abs();
        let steps = (span * 1.5).ceil() as usize + 8;
        let points: Vec<(f32, f32)> = (0..=steps)
            .map(|i| self.point(cable, i as f32 / steps as f32))
            .collect();
        let (wide, tall) = (canvas.width() as i32, canvas.height() as i32);
        // the nearest distance to the centre line for each pixel and the side it lies on
        let mut best: Vec<(f32, f32)> = vec![(f32::MAX, 0.0); (wide * tall).max(1) as usize];
        let reach = r + 1.0;
        let light = (-0.8_f32, -0.6_f32);
        for pair in points.windows(2) {
            let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = (dx * dx + dy * dy).max(0.0001);
            let (x0, x1) = (
                (ax.min(bx) - reach).floor() as i32,
                (ax.max(bx) + reach).ceil() as i32,
            );
            let (y0, y1) = (
                (ay.min(by) - reach).floor() as i32,
                (ay.max(by) + reach).ceil() as i32,
            );
            for y in y0.max(self.top())..=y1.min(tall - 1) {
                for x in x0.max(0)..=x1.min(wide - 1) {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
                    let (cx, cy) = (ax + dx * t, ay + dy * t);
                    let (ox, oy) = (px - cx, py - cy);
                    let d = (ox * ox + oy * oy).sqrt();
                    let i = (y * wide + x) as usize;
                    if d < best[i].0 {
                        let side = if d > 0.0001 {
                            (ox * light.0 + oy * light.1) / d
                        } else {
                            0.0
                        };
                        best[i] = (d, side);
                    }
                }
            }
        }
        for y in self.top().max(0)..tall {
            for x in 0..wide {
                let (d, side) = best[(y * wide + x) as usize];
                // the rim falls only on the side away from the light, a soft shadow along the
                // tube that keeps crossings apart without blurring the lit edge
                if d > reach || (d > r && side > 0.2) {
                    continue;
                }
                let colour = if d > r {
                    tube[0]
                } else {
                    // across the tube: where the surface faces the light it is lit, with a
                    // sheen part way out from the middle; the far side falls into shade
                    let facing = side * (d / r);
                    if facing > 0.5 {
                        tube[3]
                    } else if facing > 0.1 && r >= 1.4 {
                        tube[4]
                    } else if facing < -0.35 {
                        tube[1]
                    } else {
                        tube[2]
                    }
                };
                canvas.set(x, y, colour);
            }
        }
    }
}

impl Scene for Tangle {
    fn name(&self) -> &'static str {
        "tangle"
    }

    fn resize(&mut self, width: usize, height: usize, rng: &mut Rng) {
        self.width = width.max(1);
        self.height = height.max(1);
        let wanted = self
            .motion
            .scaled(count_for(self.width, self.height, 260, 3, 12))
            .min((self.width / 7).max(1));
        let gap = self.width as f32 / wanted as f32;
        let w = self.width as f32;
        let span = (self.bottom() - self.top()).max(1) as f32;
        let spread = (w / 4.0).max(2.0);
        self.cables = (0..wanted)
            .map(|i| {
                let port = gap * (i as f32 + 0.5);
                // the cable wanders across its neighbours but always stays inside the screen
                let low = 5.0 - port;
                let high = (w - 6.0 - port).max(low);
                let reach = rng.range(-spread, spread).clamp(low, high);
                let room = (port + reach).min(w - 3.0 - port - reach).max(1.0);
                let trail_room = if rng.chance(0.5) {
                    (w - 4.0 - port - reach).max(0.0)
                } else {
                    -(port + reach - 4.0).max(0.0)
                };
                let size = (span * 0.12).clamp(2.0, 9.0);
                Cable {
                    port,
                    reach,
                    amplitude: rng.range(2.0, 3.0 + w / 25.0).min(room),
                    waves: rng.range(1.2, 2.6),
                    phase: rng.range(0.0, TAU),
                    colour: i % 6,
                    knot: (span >= 14.0 && rng.chance(0.6)).then(|| {
                        let turn = if rng.chance(0.5) { 1.0 } else { -1.0 };
                        (rng.range(0.3, 0.6), size, turn)
                    }),
                    trail: trail_room.clamp(-w / 5.0, w / 5.0) * rng.range(0.4, 1.0),
                }
            })
            .collect();
        self.faulty = wanted / 2;
    }

    fn step(&mut self, dt: f32, _rng: &mut Rng) {
        self.t += f64::from(dt);
        self.tangled = (self.tangled + dt * self.motion.speed()).min(TANGLE_SECONDS * 2.0);
    }

    fn draw(&self, frame: &mut Frame, theme: &ThemeData) {
        let colours = Colours::new(theme);
        let mut canvas = Canvas::for_frame(frame, theme.background);
        self.paint(&mut canvas, &colours);
        canvas.blit(frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixels::IMAGERY_CEILING;
    use crate::scenes::paint;
    use crate::scenes::testing::{apart, check_pixel_scene, pixels};
    use crate::scenes::testing::{check_determinism, check_scene, render};
    use crate::theme;

    #[test]
    fn passes_the_shared_checks() {
        check_scene("tangle");
        check_determinism("tangle");
    }

    #[test]
    fn paints_in_pixels_in_every_theme() {
        check_pixel_scene("tangle", 40.0);
    }

    #[test]
    fn cables_open_crossed_then_knot_and_stay_on_screen() {
        let mut rng = Rng::new(3);
        let mut tangle = Tangle::new(80, 24, Motion::FULL, &mut rng);
        // from the first frame the cables have already left their ports' straight lines
        let wandered = tangle.cables.iter().any(|cable| {
            let (x, _) = tangle.point(cable, 0.5);
            (x - cable.port).abs() > 2.0
        });
        assert!(wandered);
        let opening = tangle.amount();
        for _ in 0..(3600 * 20) {
            tangle.step(0.05, &mut rng);
        }
        assert!(tangle.amount() > 0.99 && opening < 0.5);
        let frame = render(&mut tangle, 80, 24, 1);
        let theme = theme::default_theme();
        assert!(pixels(&frame).contains(&error_colour(theme)));
        // no words on the scene: the red light and the snarl say what is wrong
        assert!(frame.cells().iter().all(|c| c.is_pixel() || c.ch == ' '));
        for cable in &tangle.cables {
            for i in 0..=100 {
                let (x, y) = tangle.point(cable, i as f32 / 100.0);
                assert!((0.0..80.0).contains(&x), "{x}");
                assert!(y >= tangle.top() as f32 - 20.0 && y <= tangle.bottom() as f32);
            }
        }
    }

    #[test]
    fn the_tangle_grows_slowly() {
        let mut rng = Rng::new(8);
        let mut tangle = Tangle::new(80, 24, Motion::LIVELY, &mut rng);
        let mut last = tangle.amount();
        for _ in 0..(120 * 20) {
            tangle.step(0.05, &mut rng);
            // never more than a thirtieth of the whole tangle in a second
            assert!((tangle.amount() - last) * 20.0 < 1.0 / 30.0 * 1.5);
            last = tangle.amount();
        }
    }

    #[test]
    fn the_layers_stay_apart_in_every_theme() {
        for theme in theme::all() {
            let c = Colours::new(theme);
            apart(
                theme,
                c.wall.at(0.0),
                c.wall.at(1.0),
                0.02,
                "the wall's light and shade",
            );
            apart(
                theme,
                c.floor.at(0.0),
                c.wall.at(1.0),
                0.03,
                "the floor and the wall",
            );
            apart(
                theme,
                c.panel.1,
                c.wall.at(0.0),
                0.1,
                "the panel and the wall",
            );
            for tube in c.cables {
                apart(theme, tube[2], c.wall.at(0.5), 0.1, "a cable and the wall");
                apart(theme, tube[2], tube[0], 0.06, "a cable and its rim");
                for tone in &tube[1..] {
                    assert!(
                        theme.contrast_of(*tone) >= paint::SOLID - 0.01,
                        "{}",
                        theme.name
                    );
                }
            }
            for colour in c.wall.steps().iter().chain(c.floor.steps()) {
                assert!(
                    theme.contrast_of(*colour) < IMAGERY_CEILING,
                    "{}",
                    theme.name
                );
            }
        }
    }
}
