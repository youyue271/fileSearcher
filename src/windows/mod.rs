pub mod context_view;

use crate::windows::context_view::ContextView;

pub enum AppWindow {
    Context(ContextView),
}

impl AppWindow {
    pub fn is_open(&self) -> &bool {
        match self {
            AppWindow::Context(v) => v.is_open(),
        }
    }

    pub fn draw(&mut self, ctx: &eframe::egui::Context, search_query: &str) {
        match self {
            AppWindow::Context(v) => v.draw(ctx, search_query),
        }
    }
}
