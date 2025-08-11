use eframe::egui;

pub struct ContextView {
    id: egui::Id,
    pub path: String,
    content: String,
    open: bool,
}

impl ContextView {
    pub fn new(path: String, content: String) -> Self {
        Self {
            id: egui::Id::new(&path),
            path,
            content,
            open: true,
        }
    }

    pub fn is_open(&self) -> &bool {
        &self.open
    }

    pub fn draw(&mut self, ctx: &egui::Context, search_query: &str) {
        let mut is_open = self.open;
        egui::Window::new(&self.path)
            .id(self.id)
            .default_size([600.0, 400.0])
            .open(&mut is_open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        // A simple way to highlight search query. Case-insensitive.
                        let lower_query = search_query.to_lowercase();
                        let mut last_end = 0;
                        for (start, _)
                            in self.content.to_lowercase().match_indices(&lower_query)
                        {
                            if start > last_end {
                                ui.label(&self.content[last_end..start]);
                            }
                            let end = start + search_query.len();
                            ui.label(
                                egui::RichText::new(&self.content[start..end])
                                    .color(egui::Color32::RED)
                                    .strong(),
                            );
                            last_end = end;
                        }
                        if last_end < self.content.len() {
                            ui.label(&self.content[last_end..]);
                        }
                    });
                });
            });
        self.open = is_open;
    }
}
