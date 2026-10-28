//! The eight Zacess themes, the colour roles a scene draws with and how the active theme is found.

use std::path::PathBuf;

use crate::colour::Rgb;
pub use crate::palette::{PALETTE_VERSION, THEMES, ThemeData};

/// The theme used when nothing names one.
pub const DEFAULT_ID: &str = "deep-contrast-dark";
/// The light theme used when the switcher follows the system and the system is light.
pub const DEFAULT_LIGHT_ID: &str = "deep-contrast-light";
/// The environment variable that names a theme.
pub const ENV_VAR: &str = "ZACESS_THEME";

/// The short names the Zacess theme switcher accepts, beside the id and the full name.
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

impl ThemeData {
    /// The switcher's short name, such as `deep-dark`.
    pub fn short(&self) -> &'static str {
        SHORT_NAMES
            .iter()
            .find(|(_, id)| *id == self.id)
            .map(|(short, _)| *short)
            .unwrap_or(self.id)
    }

    pub fn red(&self) -> Rgb {
        self.ansi[1]
    }

    pub fn green(&self) -> Rgb {
        self.ansi[2]
    }

    pub fn yellow(&self) -> Rgb {
        self.ansi[3]
    }

    pub fn blue(&self) -> Rgb {
        self.ansi[4]
    }

    pub fn magenta(&self) -> Rgb {
        self.ansi[5]
    }

    pub fn cyan(&self) -> Rgb {
        self.ansi[6]
    }

    /// The bright black slot, a grey that still reads on the background.
    pub fn grey(&self) -> Rgb {
        self.ansi[8]
    }

    pub fn bright_red(&self) -> Rgb {
        self.ansi[9]
    }

    pub fn bright_green(&self) -> Rgb {
        self.ansi[10]
    }

    pub fn bright_yellow(&self) -> Rgb {
        self.ansi[11]
    }

    pub fn bright_blue(&self) -> Rgb {
        self.ansi[12]
    }

    pub fn bright_magenta(&self) -> Rgb {
        self.ansi[13]
    }

    pub fn bright_cyan(&self) -> Rgb {
        self.ansi[14]
    }

    /// The strongest text colour: white on a dark theme, near black on a light one.
    pub fn bright(&self) -> Rgb {
        self.ansi[15]
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

/// What the Zacess theme switcher left in its state file: one fixed theme or a dark and a
/// light theme to follow the system with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitcherState {
    Fixed(String),
    Auto { dark: String, light: String },
}

/// Parses the state file's text: a theme id on its own or `auto <dark id> <light id>`.
pub fn parse_switcher_state(text: &str) -> Option<SwitcherState> {
    let mut words = text.split_whitespace();
    let first = words.next()?;
    if first == "auto" {
        let dark = words.next().unwrap_or(DEFAULT_ID).to_string();
        let light = words.next().unwrap_or(DEFAULT_LIGHT_ID).to_string();
        Some(SwitcherState::Auto { dark, light })
    } else {
        Some(SwitcherState::Fixed(first.to_string()))
    }
}

/// Where the switcher keeps its state: `$XDG_STATE_HOME/zacess/theme`, otherwise
/// `~/.local/state/zacess/theme`.
pub fn switcher_state_path() -> Option<PathBuf> {
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

/// Reads the switcher's state file, if there is one.
pub fn read_switcher_state() -> Option<SwitcherState> {
    let text = std::fs::read_to_string(switcher_state_path()?).ok()?;
    parse_switcher_state(&text)
}

/// Everything that can name the theme, in order of priority.
#[derive(Clone, Debug, Default)]
pub struct Choice<'a> {
    /// The `--theme` flag.
    pub flag: Option<&'a str>,
    /// The `ZACESS_THEME` variable.
    pub env: Option<&'a str>,
    /// The switcher's state file.
    pub switcher: Option<SwitcherState>,
    /// Whether the system is in dark mode, used only when the switcher follows the system.
    pub prefers_dark: bool,
}

/// Picks the theme: the flag, then the variable, then the switcher, then the default. A name
/// given explicitly that is not a theme is an error; a stale switcher file falls back quietly.
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
    Ok(match &choice.switcher {
        Some(SwitcherState::Fixed(id)) => find(id).unwrap_or_else(default_theme),
        Some(SwitcherState::Auto { dark, light }) => {
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
    use crate::colour::MIN_CONTRAST;

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
        assert_eq!(PALETTE_VERSION, "0.4.0");
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
    fn the_flag_beats_the_variable_and_the_switcher() {
        let choice = Choice {
            flag: Some("cream"),
            env: Some("dark"),
            switcher: Some(SwitcherState::Fixed("light".into())),
            prefers_dark: true,
        };
        assert_eq!(choose(&choice).unwrap().id, "cream");
    }

    #[test]
    fn the_variable_beats_the_switcher() {
        let choice = Choice {
            flag: None,
            env: Some("Zacess Dark"),
            switcher: Some(SwitcherState::Fixed("light".into())),
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
    fn the_switcher_state_is_honoured_and_a_stale_one_falls_back() {
        let fixed = Choice {
            switcher: Some(SwitcherState::Fixed("hc-light".into())),
            ..Choice::default()
        };
        assert_eq!(choose(&fixed).unwrap().id, "high-contrast-light");
        let stale = Choice {
            switcher: Some(SwitcherState::Fixed("gone".into())),
            ..Choice::default()
        };
        assert_eq!(choose(&stale).unwrap().id, DEFAULT_ID);
        let auto = SwitcherState::Auto {
            dark: "dark".into(),
            light: "cream".into(),
        };
        let dark = Choice {
            switcher: Some(auto.clone()),
            prefers_dark: true,
            ..Choice::default()
        };
        assert_eq!(choose(&dark).unwrap().id, "dark");
        let light = Choice {
            switcher: Some(auto),
            prefers_dark: false,
            ..Choice::default()
        };
        assert_eq!(choose(&light).unwrap().id, "cream");
    }

    #[test]
    fn parses_the_switcher_file() {
        assert_eq!(
            parse_switcher_state("deep-contrast-dark\n"),
            Some(SwitcherState::Fixed("deep-contrast-dark".into()))
        );
        assert_eq!(
            parse_switcher_state("auto dark light\n"),
            Some(SwitcherState::Auto {
                dark: "dark".into(),
                light: "light".into()
            })
        );
        assert_eq!(
            parse_switcher_state("auto"),
            Some(SwitcherState::Auto {
                dark: DEFAULT_ID.into(),
                light: DEFAULT_LIGHT_ID.into()
            })
        );
        assert_eq!(parse_switcher_state("  \n"), None);
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
    fn dark_and_light_flags_match_the_backgrounds() {
        for theme in all() {
            let dark_background = theme.background.luminance() < 0.5;
            assert_eq!(theme.dark, dark_background, "{}", theme.name);
        }
    }
}
