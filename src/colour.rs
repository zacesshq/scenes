//! Colours and the WCAG contrast ratio every drawn colour is checked against.

/// The contrast ratio every drawn colour keeps against the theme background, the WCAG AA floor
/// for text.
pub const MIN_CONTRAST: f64 = 4.5;

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
    fn parses_and_prints_hex() {
        assert_eq!(Rgb::parse("#00FFff"), Some(Rgb::new(0, 255, 255)));
        assert_eq!(Rgb::parse("112233"), Some(Rgb::new(0x11, 0x22, 0x33)));
        assert_eq!(Rgb::parse("#fff"), None);
        assert_eq!(Rgb::parse("#gg0000"), None);
        assert_eq!(Rgb::new(0, 255, 255).hex(), "#00ffff");
    }
}
