//! Linux host loop: clipboard-first, safe on both X11 and Wayland.
//!
//! Copy text in another application and GSpot immediately analyses it. Accepting
//! a suggestion copies the replacement, ready to paste over the original
//! selection. This avoids global input injection, which modern Wayland
//! compositors intentionally prohibit.

use crate::ax::{AXElement, Rect};
use crate::engine::{Alert, Engine};
use crate::settings::Settings;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

const POLL: Duration = Duration::from_millis(350);
const MAX_CHARS: usize = 80_000;

#[derive(Clone, Debug, Serialize)]
pub struct HostDoc {
    pub id: String,
    pub app_name: String,
    pub bundle_id: String,
    pub pid: i32,
    pub role: String,
    pub text: String,
    pub sel_start: usize,
    pub sel_end: usize,
    pub selected_text: String,
    pub frame: Rect,
    pub visible: Rect,
    pub version: u64,
    pub active: bool,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Status {
    pub platform: String,
    pub trusted: bool,
    pub frontmost: String,
    pub frontmost_bid: String,
    pub skipped: bool,
    pub skip_built_in: bool,
    pub has_doc: bool,
    pub linting: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct UnderlineRect {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub color: String,
    pub category: String,
    #[serde(default)]
    pub sentence: bool,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Geometry {
    pub origin_x: f64,
    pub origin_y: f64,
    pub width: f64,
    pub height: f64,
    pub rects: Vec<UnderlineRect>,
    pub hovered: Option<String>,
    #[serde(default)]
    pub over_card: bool,
}

#[derive(Default)]
pub struct HostState {
    pub doc: Option<HostDoc>,
    pub element: Option<AXElement>,
    pub alerts: Vec<Alert>,
    pub all_alerts: Vec<Alert>,
    pub spot_alerts: Vec<Alert>,
    pub spot_cache: crate::features::spotcheck::Cache,
    pub spot_running: bool,
    pub spot_doc_id: String,
    pub spot_dirty: bool,
    pub spot_pending_since: Option<std::time::Instant>,
    pub spot_last_hash: u64,
    pub status: Status,
    pub geometry: Geometry,
    pub settings: Settings,
    pub history: Vec<crate::history::Session>,
    pub card_rects: Vec<Rect>,
    pub last_text_hash: u64,
    pub lint_pending_since: Option<std::time::Instant>,
    pub lint_dirty: bool,
    pub lost_focus_at: Option<std::time::Instant>,
    pub paused: bool,
    pub ui_ready: Vec<String>,
    pub ui_log: Vec<String>,
}

pub type Shared = Arc<Mutex<HostState>>;

impl HostState {
    pub fn visible_alerts(&self) -> Vec<Alert> {
        crate::features::spotcheck::merge_visible(&self.all_alerts, &self.spot_alerts, |h| {
            self.settings.dismissed.contains(&h)
        })
    }
}

pub fn spawn(app: AppHandle, shared: Shared) {
    std::thread::Builder::new()
        .name("linux-clipboard-host".into())
        .spawn(move || {
            let mut engine = Engine::new();
            let mut last = String::new();
            loop {
                let text = arboard::Clipboard::new()
                    .ok()
                    .and_then(|mut c| c.get_text().ok())
                    .unwrap_or_default();
                if text != last {
                    last = text.clone();
                    if !text.trim().is_empty() && !crate::ax::is_gspot_clipboard(&text) {
                        let truncated = text.chars().count() > MAX_CHARS;
                        let text: String = if truncated {
                            text.chars().take(MAX_CHARS).collect()
                        } else {
                            text
                        };
                        let alerts = engine.check(&text);
                        let mut state = shared.lock().unwrap();
                        let version = state.doc.as_ref().map(|d| d.version + 1).unwrap_or(1);
                        let doc = HostDoc {
                            id: format!("clipboard-{version}"),
                            app_name: "Clipboard".into(),
                            bundle_id: "linux.clipboard".into(),
                            pid: 0,
                            role: "Clipboard".into(),
                            selected_text: text.clone(),
                            sel_start: 0,
                            sel_end: text.encode_utf16().count(),
                            text,
                            frame: Rect::default(),
                            visible: Rect::default(),
                            version,
                            active: true,
                            truncated,
                        };
                        state.doc = Some(doc.clone());
                        state.element = Some(AXElement);
                        state.all_alerts = alerts;
                        state.alerts = state.visible_alerts();
                        state.status = Status {
                            platform: "linux".into(),
                            trusted: true,
                            frontmost: "Clipboard workflow".into(),
                            frontmost_bid: "linux.clipboard".into(),
                            has_doc: true,
                            ..Status::default()
                        };
                        let visible = state.alerts.clone();
                        drop(state);
                        let _ = app.emit("host://document", &doc);
                        let _ = app.emit("host://alerts", &visible);
                    }
                }
                std::thread::sleep(POLL);
            }
        })
        .expect("linux clipboard host thread");
}
