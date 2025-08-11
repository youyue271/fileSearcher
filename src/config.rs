use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: Theme,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Theme {
    Light,
    Dark,
}

impl AppSettings {
    pub fn get_visuals(&self) -> egui::Visuals {
        match self.theme {
            Theme::Light => egui::Visuals::light(),
            Theme::Dark => egui::Visuals::dark(),
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Light,
        }
    }
}