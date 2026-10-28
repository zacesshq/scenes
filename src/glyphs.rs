//! Characters that only some terminal fonts have. The Linux console and the classic Windows
//! console have no font fallback, so a character missing from their font shows as a box on every
//! frame. Block and shade characters are in those fonts; the triangles of the Zacess mark's fold
//! are not.

use std::sync::atomic::{AtomicBool, Ordering};

static BASIC: AtomicBool = AtomicBool::new(false);

/// Limits the scenes to characters every terminal font has.
pub fn set_basic(on: bool) {
    BASIC.store(on, Ordering::Relaxed);
}

/// Whether the scenes are limited to characters every terminal font has.
pub fn basic() -> bool {
    BASIC.load(Ordering::Relaxed)
}

/// The fold of the Zacess mark on a dark theme, a lower left triangle. `None` to leave the
/// corner cut square where the font may lack it.
pub fn fold_dark() -> Option<char> {
    (!basic()).then_some('\u{25e3}')
}

/// The fold of the Zacess mark on a light theme, an upper right triangle. `None` to leave the
/// corner cut square where the font may lack it.
pub fn fold_light() -> Option<char> {
    (!basic()).then_some('\u{25e5}')
}
