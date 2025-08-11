use crate::message::{AppMessage, SettingsMessage};
use crate::config::Theme;
use crossbeam_channel::Sender;
use eframe::egui;

pub struct SettingsView {
    open: bool,
    sender: Sender<AppMessage>,
    // Local state for the view
    theme: Theme,
}

impl SettingsView {
    pub fn new(sender: Sender<AppMessage>, initial_theme: Theme) -> Self {
        Self {
            open: true,
            sender,
            theme: initial_theme,
        }
    }

    pub fn is_open(&self) -> &bool {
        &self.open
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        egui::Window::new("Settings")
            .open(&mut self.open)
            .default_size([300.0, 400.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("主题设置");
                    ui.separator();

                    let mut new_theme = self.theme;
                    ui.horizontal(|ui| {
                        ui.label("外观:");
                        if ui
                            .radio_value(&mut new_theme, Theme::Light, "浅色")
                            .clicked()
                        {
                            self.theme = new_theme;
                            self.sender
                                .send(AppMessage::Settings(SettingsMessage::ThemeChanged(
                                    Theme::Light,
                                )))
                                .unwrap();
                        }
                        if ui
                            .radio_value(&mut new_theme, Theme::Dark, "深色")
                            .clicked()
                        {
                            self.theme = new_theme;
                            self.sender
                                .send(AppMessage::Settings(SettingsMessage::ThemeChanged(
                                    Theme::Dark,
                                )))
                                .unwrap();
                        }
                    });

                    ui.separator();
                    // Add other settings here in the future
                });
            });
    }
}
