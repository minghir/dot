use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_color_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut srgba = egui::Color32::from_rgba_unmultiplied(
                self.primary_color[0],
                self.primary_color[1],
                self.primary_color[2],
                self.primary_color[3],
            );
            if egui::color_picker::color_picker_color32(
                ui,
                &mut srgba,
                egui::color_picker::Alpha::OnlyBlend,
            ) {
                self.primary_color = srgba.to_array();
            }

            ui.separator();
            ui.monospace(format!(
                "HEX: #{:02X}{:02X}{:02X}{:02X}",
                self.primary_color[0],
                self.primary_color[1],
                self.primary_color[2],
                self.primary_color[3]
            ));
        });
    }
}