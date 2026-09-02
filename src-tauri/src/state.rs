use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use image::RgbaImage;

use crate::settings::Settings;

/// One monitor, frozen at the moment the hotkey fired.
pub struct Screen {
    pub index: usize,
    pub geometry: ScreenGeometry,
    pub image: RgbaImage,
    /// The image encoded once, so every overlay request is a memcpy.
    pub png: Vec<u8>,
}

/// Where the monitor sits, in the units the platform's window placement
/// expects (see `capture::grab_screens`).
#[derive(Clone, Copy, Debug)]
pub struct ScreenGeometry {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Physical pixels per geometry unit, as the platform reports it.
    pub scale_factor: f32,
}

/// The crop the user confirmed, waiting for (or sitting in) the editor.
pub struct Capture {
    pub png: Vec<u8>,
}

#[derive(Default)]
pub struct AppState {
    pub screens: Mutex<Vec<Screen>>,
    pub capture: Mutex<Option<Capture>>,
    /// Set while overlays are up; a second hotkey press is ignored.
    pub capturing: AtomicBool,
    /// Whether the editor was visible when the capture began, so a cancel
    /// can put it back.
    pub editor_was_visible: AtomicBool,
    pub settings: Mutex<Settings>,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
    }
}
