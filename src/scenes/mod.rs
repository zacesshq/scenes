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
pub mod dead_led;
pub mod desert_night;
pub mod drone_landing;
pub mod drought;
pub mod dust_storm;
pub mod erosion;
pub mod fireflies;
pub mod flood;
pub mod fog;
pub mod forest_stream;
pub mod garden;
pub mod harbour;
pub mod kettle_boil;
pub mod kettle_cold;
pub mod koi_pond;
pub mod landing_abort;
pub mod lanterns;
pub mod launch_scrub;
pub mod leak;
pub mod lily_pads;
pub mod listing_ship;
pub mod mountain_lake;
pub mod orbit;
pub mod overgrown;
pub(crate) mod paint;
pub mod paper_boats;
pub mod pcb_bring_up;
pub mod picnic;
pub mod pit_stop;
pub mod pit_stop_stuck;
pub mod rain;
pub mod ribbons;
pub mod rocket_launch;
pub mod rockslide;
pub mod rooftops;
pub mod sea;
pub mod short_circuit;
pub mod snow;
pub mod starfield;
pub mod storm;
pub mod sunflower_field;
pub mod tangle;
pub mod tea;
pub mod thin_ice;
pub mod tide_pools;
pub mod tiles;
pub mod traffic_jam;
pub mod volcano;
pub mod waves;
pub mod wilt;
pub mod windmills;
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
    /// A general scene, the original ten among them, calm enough to show while a build passes.
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

/// Every scene, in the order the help lists them: the general scenes, the original ten first,
/// then the pass scenes, then the fail scenes.
pub const SCENES: [SceneInfo; 65] = [
    general(
        "sea",
        "Fish of three sizes, bubbles, seaweed, a boat on the surface and an occasional whale, under a daylit sky in light themes",
        &[
            "Clean build. The fish approve.",
            "Not a ripple out of place.",
            "Everything compiles. Swim on.",
        ],
    ),
    general(
        "rain",
        "Rain on a lamplit evening street with puddles, a walker under an umbrella and distant lightning, an overcast afternoon in light themes",
        &[
            "A soft rain and a clean build.",
            "Nothing to fix. Listen to the rain.",
            "All green under grey skies.",
        ],
    ),
    general(
        "snow",
        "Flakes at three depths drifting over far hills and pines, settling into a bank along the bottom, by day in light themes",
        &[
            "Quiet as fresh snow.",
            "A clean run. Settle in.",
            "Not a single error in the drift.",
        ],
    ),
    general(
        "starfield",
        "Twinkling stars over a faint band of the galaxy and low hills, a crescent moon and a slow shooting star, at dawn in light themes",
        &[
            "The build passed. Make a wish anyway.",
            "Clear skies over the compiler.",
            "Every star in its place.",
        ],
    ),
    general(
        "code-rain",
        "Falling columns of characters in the theme's syntax colours, each with a bright head, over a softly shaded backdrop",
        &[
            "Every column compiles.",
            "The code falls into place.",
            "No errors in the stream.",
        ],
    ),
    general(
        "fireflies",
        "Fireflies glowing over a meadow at dusk under a moon and a great tree, by day in light themes with butterflies and flowers",
        &[
            "A quiet meadow and a clean build.",
            "Small lights, no warnings.",
            "Nothing to chase tonight.",
        ],
    ),
    general(
        "aurora",
        "Slow curtains of colour across a starry sky over snow-capped mountains, at the blue hour in light themes",
        &[
            "The sky approves of this build.",
            "Green from end to end and then some.",
            "A calm night over a clean build.",
        ],
    ),
    general(
        "campfire",
        "A campfire in a pine clearing, flames over logs in a ring of stones with rising embers, on a late afternoon in light themes",
        &[
            "Pull up a log. The build is fine.",
            "Warm and green.",
            "Nothing left to tend but the fire.",
        ],
    ),
    general(
        "waves",
        "A calm sea to the horizon under a sun, with gulls, the sun a pale disc in a warm halo in light themes",
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
        "The Zacess mark floating slowly round the middle of the screen over a soft wash with two flowing ribbons of colour",
        &[
            "Clean, clear and readable.",
            "A tidy build.",
            "All good on our side.",
        ],
    ),
    SceneInfo {
        name: "forest-stream",
        summary: forest_stream::SUMMARY,
        kind: Kind::General,
        captions: forest_stream::CAPTIONS,
    },
    SceneInfo {
        name: "desert-night",
        summary: desert_night::SUMMARY,
        kind: Kind::General,
        captions: desert_night::CAPTIONS,
    },
    SceneInfo {
        name: "rooftops",
        summary: rooftops::SUMMARY,
        kind: Kind::General,
        captions: rooftops::CAPTIONS,
    },
    SceneInfo {
        name: "lily-pads",
        summary: lily_pads::SUMMARY,
        kind: Kind::General,
        captions: lily_pads::CAPTIONS,
    },
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
        name: "sunflower-field",
        summary: sunflower_field::SUMMARY,
        kind: Kind::Pass,
        captions: sunflower_field::CAPTIONS,
    },
    SceneInfo {
        name: "mountain-lake",
        summary: mountain_lake::SUMMARY,
        kind: Kind::Pass,
        captions: mountain_lake::CAPTIONS,
    },
    SceneInfo {
        name: "tide-pools",
        summary: tide_pools::SUMMARY,
        kind: Kind::Pass,
        captions: tide_pools::CAPTIONS,
    },
    SceneInfo {
        name: "paper-boats",
        summary: paper_boats::SUMMARY,
        kind: Kind::Pass,
        captions: paper_boats::CAPTIONS,
    },
    SceneInfo {
        name: "windmills",
        summary: windmills::SUMMARY,
        kind: Kind::Pass,
        captions: windmills::CAPTIONS,
    },
    SceneInfo {
        name: "picnic",
        summary: picnic::SUMMARY,
        kind: Kind::Pass,
        captions: picnic::CAPTIONS,
    },
    SceneInfo {
        name: "drone-landing",
        summary: drone_landing::SUMMARY,
        kind: Kind::Pass,
        captions: drone_landing::CAPTIONS,
    },
    SceneInfo {
        name: "kettle-boil",
        summary: kettle_boil::SUMMARY,
        kind: Kind::Pass,
        captions: kettle_boil::CAPTIONS,
    },
    SceneInfo {
        name: "rocket-launch",
        summary: rocket_launch::SUMMARY,
        kind: Kind::Pass,
        captions: rocket_launch::CAPTIONS,
    },
    SceneInfo {
        name: "pit-stop",
        summary: pit_stop::SUMMARY,
        kind: Kind::Pass,
        captions: pit_stop::CAPTIONS,
    },
    SceneInfo {
        name: "pcb-bring-up",
        summary: pcb_bring_up::SUMMARY,
        kind: Kind::Pass,
        captions: pcb_bring_up::CAPTIONS,
    },
    SceneInfo {
        name: "storm",
        summary: storm::SUMMARY,
        kind: Kind::Fail,
        captions: storm::CAPTIONS,
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
    SceneInfo {
        name: "volcano",
        summary: volcano::SUMMARY,
        kind: Kind::Fail,
        captions: volcano::CAPTIONS,
    },
    SceneInfo {
        name: "rockslide",
        summary: rockslide::SUMMARY,
        kind: Kind::Fail,
        captions: rockslide::CAPTIONS,
    },
    SceneInfo {
        name: "dust-storm",
        summary: dust_storm::SUMMARY,
        kind: Kind::Fail,
        captions: dust_storm::CAPTIONS,
    },
    SceneInfo {
        name: "thin-ice",
        summary: thin_ice::SUMMARY,
        kind: Kind::Fail,
        captions: thin_ice::CAPTIONS,
    },
    SceneInfo {
        name: "short-circuit",
        summary: short_circuit::SUMMARY,
        kind: Kind::Fail,
        captions: short_circuit::CAPTIONS,
    },
    SceneInfo {
        name: "overgrown",
        summary: overgrown::SUMMARY,
        kind: Kind::Fail,
        captions: overgrown::CAPTIONS,
    },
    SceneInfo {
        name: "landing-abort",
        summary: landing_abort::SUMMARY,
        kind: Kind::Fail,
        captions: landing_abort::CAPTIONS,
    },
    SceneInfo {
        name: "kettle-cold",
        summary: kettle_cold::SUMMARY,
        kind: Kind::Fail,
        captions: kettle_cold::CAPTIONS,
    },
    SceneInfo {
        name: "launch-scrub",
        summary: launch_scrub::SUMMARY,
        kind: Kind::Fail,
        captions: launch_scrub::CAPTIONS,
    },
    SceneInfo {
        name: "pit-stop-stuck",
        summary: pit_stop_stuck::SUMMARY,
        kind: Kind::Fail,
        captions: pit_stop_stuck::CAPTIONS,
    },
    SceneInfo {
        name: "dead-led",
        summary: dead_led::SUMMARY,
        kind: Kind::Fail,
        captions: dead_led::CAPTIONS,
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
        "storm" => Box::new(storm::Storm::new(width, height, motion, rng)),
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
        "sunflower-field" => Box::new(sunflower_field::SunflowerField::new(
            width, height, motion, rng,
        )),
        "mountain-lake" => Box::new(mountain_lake::MountainLake::new(width, height, motion, rng)),
        "tide-pools" => Box::new(tide_pools::TidePools::new(width, height, motion, rng)),
        "paper-boats" => Box::new(paper_boats::PaperBoats::new(width, height, motion, rng)),
        "windmills" => Box::new(windmills::Windmills::new(width, height, motion, rng)),
        "picnic" => Box::new(picnic::Picnic::new(width, height, motion, rng)),
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
        "volcano" => Box::new(volcano::Volcano::new(width, height, motion, rng)),
        "rockslide" => Box::new(rockslide::Rockslide::new(width, height, motion, rng)),
        "dust-storm" => Box::new(dust_storm::DustStorm::new(width, height, motion, rng)),
        "thin-ice" => Box::new(thin_ice::ThinIce::new(width, height, motion, rng)),
        "short-circuit" => Box::new(short_circuit::ShortCircuit::new(width, height, motion, rng)),
        "overgrown" => Box::new(overgrown::Overgrown::new(width, height, motion, rng)),
        "forest-stream" => Box::new(forest_stream::ForestStream::new(width, height, motion, rng)),
        "desert-night" => Box::new(desert_night::DesertNight::new(width, height, motion, rng)),
        "rooftops" => Box::new(rooftops::Rooftops::new(width, height, motion, rng)),
        "lily-pads" => Box::new(lily_pads::LilyPads::new(width, height, motion, rng)),
        "drone-landing" => Box::new(drone_landing::DroneLanding::new(width, height, motion, rng)),
        "landing-abort" => Box::new(landing_abort::new(width, height, motion, rng)),
        "kettle-boil" => Box::new(kettle_boil::Kettle::new(width, height, motion, rng)),
        "kettle-cold" => Box::new(kettle_cold::new(width, height, motion, rng)),
        "rocket-launch" => Box::new(rocket_launch::RocketLaunch::new(width, height, motion, rng)),
        "launch-scrub" => Box::new(launch_scrub::LaunchScrub::new(width, height, motion, rng)),
        "pit-stop" => Box::new(pit_stop::PitStop::new(width, height, motion, rng)),
        "pit-stop-stuck" => Box::new(pit_stop_stuck::PitStopStuck::new(
            width, height, motion, rng,
        )),
        "pcb-bring-up" => Box::new(pcb_bring_up::PcbBringUp::new(width, height, motion, rng)),
        "dead-led" => Box::new(dead_led::DeadLed::new(width, height, motion, rng)),
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
pub(crate) mod testing;

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
