//! Linux replacement path.
//!
//! We never synthesize keyboard events here.  Wayland deliberately prevents
//! arbitrary applications from doing that, so accepting a suggestion copies
//! the replacement to the clipboard.  The user pastes it into the still
//! selected text in their editor.

use crate::ax::AXElement;
use crate::textmap::{slice_utf16, splice_utf16};
use anyhow::{anyhow, Result};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Clipboard,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyResult {
    pub method: Method,
    pub new_text: String,
}

/// Copy a replacement to the clipboard. `el` keeps this signature aligned with
/// the macOS backend.
pub fn replace_range(
    _: &AXElement,
    _: i32,
    loc: usize,
    len: usize,
    replacement: &str,
    before: &str,
) -> Result<ApplyResult> {
    let original = slice_utf16(before, loc, loc + len);
    if original.is_empty() && len != 0 {
        return Err(anyhow!(
            "The copied text changed. Copy it again, then retry."
        ));
    }
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| anyhow!("Clipboard is unavailable: {e}"))?;
    clipboard
        .set_text(replacement.to_owned())
        .map_err(|e| anyhow!("Could not copy the replacement: {e}"))?;
    crate::ax::mark_gspot_clipboard(replacement);
    Ok(ApplyResult {
        method: Method::Clipboard,
        new_text: splice_utf16(before, loc, len, replacement),
    })
}
