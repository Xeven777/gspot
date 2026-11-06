//! Linux platform helpers.
//!
//! Linux does not have a system-wide permission dialog equivalent to macOS
//! Accessibility.  The first Linux release deliberately uses the clipboard as
//! its cross-desktop host boundary: it works in X11 and Wayland without asking
//! a compositor to permit synthetic input.  AT-SPI can be added behind this
//! module later for desktop-specific focused-field and geometry support.

use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

static LAST_GSPOT_CLIPBOARD: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn gspot_clipboard() -> &'static Mutex<Option<String>> {
    LAST_GSPOT_CLIPBOARD.get_or_init(|| Mutex::new(None))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// A clipboard session is the Linux equivalent of the current host handle.
/// It is intentionally opaque so callers cannot accidentally assume macOS AX
/// semantics on a Wayland session.
#[derive(Clone, Debug, Default)]
pub struct AXElement;

pub fn is_trusted() -> bool {
    // Clipboard access is mediated by the desktop; there is no app-level
    // accessibility grant to request for this workflow.
    true
}

pub fn prompt_trust() -> bool {
    true
}

pub fn mouse_location_top_left() -> Option<(f64, f64)> {
    None
}

/// Synthetic global pointer events are intentionally unsupported on Linux.
pub fn post_mouse_move(_: f64, _: f64) -> bool {
    false
}

/// Prevent the host loop from treating a replacement GSpot just copied as a
/// fresh document on its next clipboard poll.
pub fn mark_gspot_clipboard(text: &str) {
    *gspot_clipboard().lock().unwrap() = Some(text.to_owned());
}

pub fn is_gspot_clipboard(text: &str) -> bool {
    let mut last = gspot_clipboard().lock().unwrap();
    if last.as_deref() == Some(text) {
        *last = None;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gspot_clipboard_marker_is_consumed_once() {
        mark_gspot_clipboard("replacement");
        assert!(is_gspot_clipboard("replacement"));
        assert!(!is_gspot_clipboard("replacement"));
    }
}
