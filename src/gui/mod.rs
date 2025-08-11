pub mod context_view;
pub mod settings_view;

use self::context_view::ContextView;
use self::settings_view::SettingsView;

pub enum AppWindow {
    Context(ContextView),
    Settings(SettingsView),
}

impl AppWindow {
    pub fn is_open(&self) -> &bool {
        match self {
            AppWindow::Context(v) => v.is_open(),
            AppWindow::Settings(v) => v.is_open(),
        }
    }

    pub fn draw(&mut self, ctx: &eframe::egui::Context, search_query: &str) {
        match self {
            AppWindow::Context(v) => v.draw(ctx, search_query),
            AppWindow::Settings(v) => v.draw(ctx),
        }
    }
}
