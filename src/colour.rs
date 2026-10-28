//! Colours and the WCAG contrast ratio every drawn colour is checked against.

/// The contrast ratio every drawn colour keeps against the theme background, the WCAG AA floor
/// for text.
pub const MIN_CONTRAST: f64 = 4.5;

/// The minimum contrast between two colours that touch inside one shape, such as the band and
/// the page of the Zacess mark, the WCAG non-text contrast floor.
pub const MIN_ADJACENT_CONTRAST: f64 = 3.0;

/// How many colours the terminal can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourDepth {
    /// 24 bit colour, sent as exact RGB.
    TrueColour,
    /// The xterm 256 colour table, for terminals without 24 bit colour.
    Ansi256,
}

impl ColourDepth {
    /// Parses the `--colour` value.
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "truecolor" | "truecolour" | "24bit" | "24-bit" | "rgb" => Some(Self::TrueColour),
            "256" | "ansi256" | "8bit" | "8-bit" => Some(Self::Ansi256),
            _ => None,
        }
    }

    /// The depth a terminal advertises through its environment. `COLORTERM` is the common signal;
    /// a few terminals that support 24 bit colour without setting it are known by name. Anything
    /// else gets the 256 colour table, which every current terminal draws faithfully.
    pub fn detect(var: impl Fn(&str) -> Option<String>) -> Self {
        let colorterm = var("COLORTERM").unwrap_or_default().to_ascii_lowercase();
        if colorterm.contains("truecolor") || colorterm.contains("24bit") {
            return Self::TrueColour;
        }
        let term = var("TERM").unwrap_or_default().to_ascii_lowercase();
        if term.contains("truecolor") || term.contains("24bit") || term.ends_with("-direct") {
            return Self::TrueColour;
        }
        if var("WT_SESSION").is_some_and(|v| !v.is_empty()) {
            return Self::TrueColour;
        }
        let program = var("TERM_PROGRAM").unwrap_or_default();
        const TRUE_COLOUR_PROGRAMS: [&str; 8] = [
            "iTerm.app",
            "WezTerm",
            "vscode",
            "ghostty",
            "Hyper",
            "Tabby",
            "WarpTerminal",
            "rio",
        ];
        if TRUE_COLOUR_PROGRAMS.contains(&program.as_str()) {
            return Self::TrueColour;
        }
        Self::Ansi256
    }
}

/// The colour of one entry in the xterm 256 colour table, from 16 upwards. Entries 0 to 15 are
/// left out because each terminal theme sets them differently.
pub fn xterm_colour(index: u8) -> Rgb {
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    if index >= 232 {
        let grey = 8 + 10 * (index - 232);
        Rgb::new(grey, grey, grey)
    } else {
        let i = index.saturating_sub(16);
        Rgb::new(
            LEVELS[(i / 36) as usize],
            LEVELS[((i / 6) % 6) as usize],
            LEVELS[(i % 6) as usize],
        )
    }
}

/// The xterm 256 colour entry closest to `colour` that still keeps `floor` against `background`.
/// When no entry keeps the floor the closest entry overall is used.
pub fn nearest_256(colour: Rgb, background: Rgb, floor: f64) -> u8 {
    let distance = |c: Rgb| {
        let dr = i32::from(c.r) - i32::from(colour.r);
        let dg = i32::from(c.g) - i32::from(colour.g);
        let db = i32::from(c.b) - i32::from(colour.b);
        // weighted towards green, which the eye is most sensitive to
        2 * dr * dr + 4 * dg * dg + 3 * db * db
    };
    let best = |keep_floor: bool| {
        (16..=255u8)
            .filter(|&i| !keep_floor || xterm_colour(i).contrast(background) >= floor)
            .min_by_key(|&i| distance(xterm_colour(i)))
    };
    best(true).or_else(|| best(false)).unwrap_or(16)
}

/// A 24 bit colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Builds a colour from a hex value such as `0x00ffff`.
    pub const fn from_hex(hex: u32) -> Self {
        Self::new(
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        )
    }

    /// Parses `#rrggbb` or `rrggbb`.
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.trim().trim_start_matches('#');
        if digits.len() != 6 {
            return None;
        }
        u32::from_str_radix(digits, 16).ok().map(Self::from_hex)
    }

    /// The colour as `#rrggbb`.
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Relative luminance as WCAG 2 defines it, from 0 for black to 1 for white.
    pub fn luminance(self) -> f64 {
        fn channel(value: u8) -> f64 {
            let c = f64::from(value) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// The WCAG contrast ratio between two colours, from 1 to 21.
    pub fn contrast(self, other: Rgb) -> f64 {
        let a = self.luminance();
        let b = other.luminance();
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_twenty_one_to_one() {
        let ratio = Rgb::from_hex(0x000000).contrast(Rgb::from_hex(0xffffff));
        assert!((ratio - 21.0).abs() < 0.01, "{ratio}");
    }

    #[test]
    fn the_aa_boundary_grey_measures_as_expected() {
        // #767676 on white is the classic 4.54:1 example
        let ratio = Rgb::from_hex(0x767676).contrast(Rgb::from_hex(0xffffff));
        assert!((ratio - 4.54).abs() < 0.01, "{ratio}");
        assert!(ratio >= MIN_CONTRAST);
    }

    #[test]
    fn contrast_is_symmetric() {
        let a = Rgb::from_hex(0x0049a3);
        let b = Rgb::from_hex(0xffffff);
        assert_eq!(a.contrast(b), b.contrast(a));
    }

    #[test]
    fn the_xterm_table_has_its_cube_and_greys() {
        assert_eq!(xterm_colour(16), Rgb::new(0, 0, 0));
        assert_eq!(xterm_colour(231), Rgb::new(255, 255, 255));
        assert_eq!(xterm_colour(226), Rgb::new(255, 255, 0));
        assert_eq!(xterm_colour(232), Rgb::new(8, 8, 8));
        assert_eq!(xterm_colour(255), Rgb::new(238, 238, 238));
    }

    #[test]
    fn the_nearest_256_colour_keeps_the_floor() {
        let black = Rgb::from_hex(0x000000);
        assert_eq!(nearest_256(Rgb::from_hex(0xffff00), black, 4.5), 226);
        let white = Rgb::from_hex(0xffffff);
        // a yellow right at the floor maps to a darker entry rather than a lighter one
        let yellow = Rgb::from_hex(0x907300);
        let index = nearest_256(yellow, white, 4.5);
        assert!(xterm_colour(index).contrast(white) >= 4.5, "{index}");
        let index = nearest_256(yellow, white, 7.0);
        assert!(xterm_colour(index).contrast(white) >= 7.0, "{index}");
    }

    #[test]
    fn colour_depth_is_detected_from_the_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == name)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(
            ColourDepth::detect(env(&[("COLORTERM", "truecolor")])),
            ColourDepth::TrueColour
        );
        assert_eq!(
            ColourDepth::detect(env(&[("WT_SESSION", "abc")])),
            ColourDepth::TrueColour
        );
        assert_eq!(
            ColourDepth::detect(env(&[("TERM_PROGRAM", "iTerm.app")])),
            ColourDepth::TrueColour
        );
        assert_eq!(
            ColourDepth::detect(env(&[("TERM_PROGRAM", "Apple_Terminal")])),
            ColourDepth::Ansi256
        );
        assert_eq!(
            ColourDepth::detect(env(&[("TERM", "linux")])),
            ColourDepth::Ansi256
        );
        assert_eq!(ColourDepth::parse("256"), Some(ColourDepth::Ansi256));
        assert_eq!(
            ColourDepth::parse("TrueColor"),
            Some(ColourDepth::TrueColour)
        );
        assert_eq!(ColourDepth::parse("16"), None);
    }

    #[test]
    fn parses_and_prints_hex() {
        assert_eq!(Rgb::parse("#00FFff"), Some(Rgb::new(0, 255, 255)));
        assert_eq!(Rgb::parse("112233"), Some(Rgb::new(0x11, 0x22, 0x33)));
        assert_eq!(Rgb::parse("#fff"), None);
        assert_eq!(Rgb::parse("#gg0000"), None);
        assert_eq!(Rgb::new(0, 255, 255).hex(), "#00ffff");
    }
}
