//! The eight Zacess themes, the colour roles a scene draws with and how the active theme is found.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::colour::{MIN_CONTRAST, Rgb};
pub use crate::palette::{PALETTE_VERSION, THEMES, ThemeData};

/// The theme used when nothing names one.
pub const DEFAULT_ID: &str = "deep-contrast-dark";
/// The light theme used when the state file follows the system and the system is light.
pub const DEFAULT_LIGHT_ID: &str = "deep-contrast-light";
/// The environment variable that names a theme.
pub const ENV_VAR: &str = "ZACESS_THEME";

/// Short names accepted beside the id and the full name.
const SHORT_NAMES: [(&str, &str); 8] = [
    ("deep-dark", "deep-contrast-dark"),
    ("deep-light", "deep-contrast-light"),
    ("hc-dark", "high-contrast-dark"),
    ("hc-light", "high-contrast-light"),
    ("dark", "dark"),
    ("light", "light"),
    ("yellow", "yellow-on-black"),
    ("cream", "cream"),
];

/// The contrast floor for the High Contrast and Deep Contrast themes, the WCAG AAA level for
/// text. People choose those themes for strength, so a scene keeps it there too.
pub const STRONG_CONTRAST: f64 = 7.0;

/// Hue and saturation of a colour, hue in degrees.
fn hue_and_saturation(colour: Rgb) -> (f32, f32) {
    let r = f32::from(colour.r) / 255.0;
    let g = f32::from(colour.g) / 255.0;
    let b = f32::from(colour.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let saturation = if max > 0.0 { delta / max } else { 0.0 };
    if delta <= f32::EPSILON {
        return (0.0, saturation);
    }
    let hue = if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, saturation)
}

/// The sixteen terminal colours of a theme with any colour below the theme's floor swapped for
/// the closest palette colour that keeps it: first the bright or plain twin of the same slot,
/// then the palette colour nearest in hue.
fn fit_ansi(theme: &ThemeData) -> [Rgb; 16] {
    let floor = theme.floor();
    let keeps = |c: Rgb| theme.contrast_of(c) >= floor;
    let mut candidates = vec![
        theme.foreground,
        theme.muted,
        theme.accent,
        theme.link,
        theme.error,
        theme.warning,
        theme.info,
    ];
    candidates.extend_from_slice(&theme.syntax);
    candidates.retain(|&c| keeps(c));
    let mut fitted = theme.ansi;
    for (slot, colour) in fitted.iter_mut().enumerate().skip(1) {
        if keeps(*colour) {
            continue;
        }
        let twin = match slot {
            1..=6 => Some(theme.ansi[slot + 8]),
            9..=14 => Some(theme.ansi[slot - 8]),
            _ => None,
        };
        if let Some(twin) = twin.filter(|&c| keeps(c)) {
            *colour = twin;
            continue;
        }
        let (hue, saturation) = hue_and_saturation(*colour);
        let grey = saturation < 0.15;
        *colour = candidates
            .iter()
            .copied()
            .min_by(|&a, &b| {
                let score = |c: Rgb| {
                    let (h, s) = hue_and_saturation(c);
                    if grey {
                        s
                    } else {
                        let d = (h - hue).abs();
                        d.min(360.0 - d) / 180.0 + if s < 0.15 { 1.0 } else { 0.0 }
                    }
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap_or(theme.foreground);
    }
    fitted
}

impl ThemeData {
    /// The contrast every colour a scene draws keeps against this theme's background: 7 to 1 on
    /// the High Contrast and Deep Contrast themes, 4.5 to 1 on the others.
    pub fn floor(&self) -> f64 {
        if self.name.contains("High Contrast") || self.name.contains("Deep Contrast") {
            STRONG_CONTRAST
        } else {
            MIN_CONTRAST
        }
    }

    /// A terminal colour, swapped for a stronger palette colour when it is below the floor.
    fn fit(&self, slot: usize) -> Rgb {
        static FITTED: OnceLock<Vec<[Rgb; 16]>> = OnceLock::new();
        let table = FITTED.get_or_init(|| THEMES.iter().map(fit_ansi).collect());
        THEMES
            .iter()
            .position(|t| t.id == self.id)
            .map_or(self.ansi[slot], |i| table[i][slot])
    }

    /// The faintest shade character a scene draws. A light shade covers about a quarter of the
    /// cell, so it reads fainter than its measured colour; the High Contrast and Deep Contrast
    /// themes use the medium shade instead.
    pub fn faint_shade(&self) -> char {
        if self.floor() >= STRONG_CONTRAST {
            '\u{2592}'
        } else {
            '\u{2591}'
        }
    }

    /// The short name, such as `deep-dark`.
    pub fn short(&self) -> &'static str {
        SHORT_NAMES
            .iter()
            .find(|(_, id)| *id == self.id)
            .map(|(short, _)| *short)
            .unwrap_or(self.id)
    }

    pub fn red(&self) -> Rgb {
        self.fit(1)
    }

    pub fn green(&self) -> Rgb {
        self.fit(2)
    }

    pub fn yellow(&self) -> Rgb {
        self.fit(3)
    }

    pub fn blue(&self) -> Rgb {
        self.fit(4)
    }

    pub fn magenta(&self) -> Rgb {
        self.fit(5)
    }

    pub fn cyan(&self) -> Rgb {
        self.fit(6)
    }

    /// The bright black slot, a grey that still reads on the background.
    pub fn grey(&self) -> Rgb {
        self.fit(8)
    }

    pub fn bright_red(&self) -> Rgb {
        self.fit(9)
    }

    pub fn bright_green(&self) -> Rgb {
        self.fit(10)
    }

    pub fn bright_yellow(&self) -> Rgb {
        self.fit(11)
    }

    pub fn bright_blue(&self) -> Rgb {
        self.fit(12)
    }

    pub fn bright_magenta(&self) -> Rgb {
        self.fit(13)
    }

    pub fn bright_cyan(&self) -> Rgb {
        self.fit(14)
    }

    /// The strongest text colour: white on a dark theme, near black on a light one.
    pub fn bright(&self) -> Rgb {
        self.fit(15)
    }

    pub fn keyword(&self) -> Rgb {
        self.syntax[0]
    }

    pub fn control(&self) -> Rgb {
        self.syntax[1]
    }

    pub fn function(&self) -> Rgb {
        self.syntax[2]
    }

    pub fn type_name(&self) -> Rgb {
        self.syntax[3]
    }

    pub fn string(&self) -> Rgb {
        self.syntax[4]
    }

    pub fn number(&self) -> Rgb {
        self.syntax[5]
    }

    pub fn comment(&self) -> Rgb {
        self.syntax[6]
    }

    pub fn property(&self) -> Rgb {
        self.syntax[7]
    }

    pub fn regex(&self) -> Rgb {
        self.syntax[8]
    }

    /// Every colour a scene may draw with. ANSI black is left out because in Zacess Deep
    /// Contrast Dark it is the background itself.
    pub fn drawable(&self) -> Vec<Rgb> {
        let mut colours = vec![
            self.foreground,
            self.muted,
            self.accent,
            self.link,
            self.cursor,
            self.error,
            self.warning,
            self.info,
        ];
        colours.extend_from_slice(&self.syntax);
        colours.extend_from_slice(&self.ansi[1..]);
        colours.sort_unstable();
        colours.dedup();
        colours
    }

    /// The contrast of a colour against this theme's background.
    pub fn contrast_of(&self, colour: Rgb) -> f64 {
        colour.contrast(self.background)
    }
}

/// Every theme in the palette's order.
pub fn all() -> &'static [ThemeData] {
    &THEMES
}

pub fn default_theme() -> &'static ThemeData {
    by_id(DEFAULT_ID).expect("the default theme is in the palette")
}

pub fn by_id(id: &str) -> Option<&'static ThemeData> {
    THEMES.iter().find(|t| t.id == id)
}

fn normalise(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '_' { '-' } else { c })
        .collect()
}

/// Finds a theme by id, short name or full name, ignoring case and the `Zacess` prefix.
pub fn find(name: &str) -> Option<&'static ThemeData> {
    let key = normalise(name);
    let key = key.strip_prefix("zacess-").unwrap_or(&key);
    if key.is_empty() {
        return None;
    }
    THEMES.iter().find(|t| {
        t.id == key || t.short() == key || normalise(t.name).strip_prefix("zacess-") == Some(key)
    })
}

/// What the theme state file holds: one fixed theme or a dark and a light theme to follow the
/// system with. Any tool that sets the terminal theme can write it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThemeState {
    Fixed(String),
    Auto { dark: String, light: String },
}

/// Parses the state file's text: a theme id on its own or `auto <dark id> <light id>`.
pub fn parse_state_file(text: &str) -> Option<ThemeState> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    if first == "auto" {
        let dark = words.next().unwrap_or(DEFAULT_ID).to_string();
        let light = words.next().unwrap_or(DEFAULT_LIGHT_ID).to_string();
        Some(ThemeState::Auto { dark, light })
    } else {
        Some(ThemeState::Fixed(first.to_string()))
    }
}

/// Where the theme state file lives: `$XDG_STATE_HOME/zacess/theme`, otherwise
/// `~/.local/state/zacess/theme`.
pub fn state_file_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => {
            let home = std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .filter(|v| !v.is_empty())?;
            PathBuf::from(home).join(".local").join("state")
        }
    };
    Some(base.join("zacess").join("theme"))
}

/// Reads the theme state file, if there is one.
pub fn read_state_file() -> Option<ThemeState> {
    let text = std::fs::read_to_string(state_file_path()?).ok()?;
    parse_state_file(&text)
}

/// Everything that can name the theme, in order of priority.
#[derive(Clone, Debug, Default)]
pub struct Choice<'a> {
    /// The `--theme` flag.
    pub flag: Option<&'a str>,
    /// The `ZACESS_THEME` variable.
    pub env: Option<&'a str>,
    /// The theme state file.
    pub state_file: Option<ThemeState>,
    /// Whether the system is in dark mode, used only when the state file follows the system.
    pub prefers_dark: bool,
}

/// Picks the theme: the flag, then the variable, then the state file, then the default. A name
/// given explicitly that is not a theme is an error; a stale state file falls back quietly.
pub fn choose(choice: &Choice) -> Result<&'static ThemeData, String> {
    if let Some(name) = choice.flag.map(str::trim).filter(|n| !n.is_empty()) {
        return find(name).ok_or_else(|| {
            format!("\"{name}\" is not a Zacess theme. Run with --list-themes to see the names.")
        });
    }
    if let Some(name) = choice.env.map(str::trim).filter(|n| !n.is_empty()) {
        return find(name).ok_or_else(|| {
            format!(
                "{ENV_VAR} is \"{name}\", which is not a Zacess theme. Run with --list-themes to see the names."
            )
        });
    }
    Ok(match &choice.state_file {
        Some(ThemeState::Fixed(id)) => find(id).unwrap_or_else(default_theme),
        Some(ThemeState::Auto { dark, light }) => {
            if choice.prefers_dark {
                find(dark).unwrap_or_else(default_theme)
            } else {
                find(light)
                    .or_else(|| by_id(DEFAULT_LIGHT_ID))
                    .unwrap_or_else(default_theme)
            }
        }
        None => default_theme(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 8] = [
        "Zacess Light",
        "Zacess Dark",
        "Zacess Yellow on Black",
        "Zacess Cream",
        "Zacess High Contrast Dark",
        "Zacess High Contrast Light",
        "Zacess Deep Contrast Dark",
        "Zacess Deep Contrast Light",
    ];

    #[test]
    fn the_eight_themes_keep_the_palette_names() {
        let names: Vec<&str> = all().iter().map(|t| t.name).collect();
        assert_eq!(names, NAMES);
        assert_eq!(PALETTE_VERSION, "0.5.0");
    }

    #[test]
    fn every_theme_has_a_short_name_of_its_own() {
        let mut shorts: Vec<&str> = all().iter().map(ThemeData::short).collect();
        shorts.sort_unstable();
        shorts.dedup();
        assert_eq!(shorts.len(), 8);
        assert_eq!(by_id("deep-contrast-dark").unwrap().short(), "deep-dark");
        assert_eq!(by_id("yellow-on-black").unwrap().short(), "yellow");
    }

    #[test]
    fn finds_by_id_short_name_and_full_name() {
        let deep = by_id("deep-contrast-dark").unwrap();
        for name in [
            "deep-contrast-dark",
            "deep-dark",
            "Zacess Deep Contrast Dark",
            "deep contrast dark",
            "DEEP_CONTRAST_DARK",
            "  zacess-deep-dark ",
        ] {
            assert_eq!(find(name).map(|t| t.id), Some(deep.id), "{name}");
        }
        assert_eq!(find("yellow").map(|t| t.id), Some("yellow-on-black"));
        assert_eq!(
            find("Yellow on Black").map(|t| t.id),
            Some("yellow-on-black")
        );
        assert_eq!(find("hc-light").map(|t| t.id), Some("high-contrast-light"));
        assert!(find("solarized").is_none());
        assert!(find("").is_none());
    }

    #[test]
    fn the_default_is_deep_contrast_dark() {
        assert_eq!(default_theme().name, "Zacess Deep Contrast Dark");
        assert_eq!(choose(&Choice::default()).unwrap().id, DEFAULT_ID);
    }

    #[test]
    fn the_flag_beats_the_variable_and_the_state_file() {
        let choice = Choice {
            flag: Some("cream"),
            env: Some("dark"),
            state_file: Some(ThemeState::Fixed("light".into())),
            prefers_dark: true,
        };
        assert_eq!(choose(&choice).unwrap().id, "cream");
    }

    #[test]
    fn the_variable_beats_the_state_file() {
        let choice = Choice {
            flag: None,
            env: Some("Zacess Dark"),
            state_file: Some(ThemeState::Fixed("light".into())),
            prefers_dark: true,
        };
        assert_eq!(choose(&choice).unwrap().id, "dark");
    }

    #[test]
    fn an_unknown_explicit_name_is_an_error() {
        let flag = Choice {
            flag: Some("nope"),
            ..Choice::default()
        };
        assert!(choose(&flag).unwrap_err().contains("--list-themes"));
        let env = Choice {
            env: Some("nope"),
            ..Choice::default()
        };
        assert!(choose(&env).unwrap_err().contains(ENV_VAR));
        let blank = Choice {
            flag: Some("  "),
            env: Some(""),
            ..Choice::default()
        };
        assert_eq!(choose(&blank).unwrap().id, DEFAULT_ID);
    }

    #[test]
    fn the_state_file_is_honoured_and_a_stale_one_falls_back() {
        let fixed = Choice {
            state_file: Some(ThemeState::Fixed("hc-light".into())),
            ..Choice::default()
        };
        assert_eq!(choose(&fixed).unwrap().id, "high-contrast-light");
        let stale = Choice {
            state_file: Some(ThemeState::Fixed("gone".into())),
            ..Choice::default()
        };
        assert_eq!(choose(&stale).unwrap().id, DEFAULT_ID);
        let auto = ThemeState::Auto {
            dark: "dark".into(),
            light: "cream".into(),
        };
        let dark = Choice {
            state_file: Some(auto.clone()),
            prefers_dark: true,
            ..Choice::default()
        };
        assert_eq!(choose(&dark).unwrap().id, "dark");
        let light = Choice {
            state_file: Some(auto),
            prefers_dark: false,
            ..Choice::default()
        };
        assert_eq!(choose(&light).unwrap().id, "cream");
    }

    #[test]
    fn parses_the_state_file() {
        assert_eq!(
            parse_state_file("deep-contrast-dark\n"),
            Some(ThemeState::Fixed("deep-contrast-dark".into()))
        );
        assert_eq!(
            parse_state_file("auto dark light\n"),
            Some(ThemeState::Auto {
                dark: "dark".into(),
                light: "light".into()
            })
        );
        assert_eq!(
            parse_state_file("auto"),
            Some(ThemeState::Auto {
                dark: DEFAULT_ID.into(),
                light: DEFAULT_LIGHT_ID.into()
            })
        );
        assert_eq!(parse_state_file("  \n"), None);
    }

    #[test]
    fn every_drawable_colour_keeps_the_contrast_floor_in_every_theme() {
        for theme in all() {
            for colour in theme.drawable() {
                let ratio = theme.contrast_of(colour);
                assert!(
                    ratio >= MIN_CONTRAST,
                    "{} {} is {ratio:.2}:1 on {}",
                    theme.name,
                    colour.hex(),
                    theme.background.hex()
                );
            }
        }
    }

    #[test]
    fn every_colour_role_keeps_the_theme_floor() {
        for theme in all() {
            let roles = [
                theme.red(),
                theme.green(),
                theme.yellow(),
                theme.blue(),
                theme.magenta(),
                theme.cyan(),
                theme.grey(),
                theme.bright_red(),
                theme.bright_green(),
                theme.bright_yellow(),
                theme.bright_blue(),
                theme.bright_magenta(),
                theme.bright_cyan(),
                theme.bright(),
                theme.foreground,
                theme.muted,
            ];
            for colour in roles.into_iter().chain(theme.syntax) {
                let ratio = theme.contrast_of(colour);
                assert!(
                    ratio >= theme.floor(),
                    "{} {} is {ratio:.2}:1, below {}:1",
                    theme.name,
                    colour.hex(),
                    theme.floor()
                );
                assert!(theme.drawable().contains(&colour), "{}", colour.hex());
            }
        }
    }

    #[test]
    fn the_strong_themes_get_the_higher_floor() {
        assert_eq!(
            by_id("deep-contrast-light").unwrap().floor(),
            STRONG_CONTRAST
        );
        assert_eq!(
            by_id("high-contrast-dark").unwrap().floor(),
            STRONG_CONTRAST
        );
        assert_eq!(by_id("cream").unwrap().floor(), MIN_CONTRAST);
        // a colour already strong enough is left alone
        let deep = by_id("deep-contrast-dark").unwrap();
        assert_eq!(deep.bright_yellow(), deep.ansi[11]);
    }

    #[test]
    fn dark_and_light_flags_match_the_backgrounds() {
        for theme in all() {
            let dark_background = theme.background.luminance() < 0.5;
            assert_eq!(theme.dark, dark_background, "{}", theme.name);
        }
    }
}
