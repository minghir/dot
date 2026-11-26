use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_history_panel(&mut self, ui: &mut egui::Ui) {
        let undo_len = self.frames[self.active_frame].undo_stack.len();
        let redo_len = self.frames[self.active_frame].redo_stack.len();

        ui.horizontal(|ui| {
            if ui
                .add_enabled(undo_len > 0, egui::Button::new("Undo"))
                .clicked()
            {
                self.edit_undo();
            }
            if ui
                .add_enabled(redo_len > 0, egui::Button::new("Redo"))
                .clicked()
            {
                self.edit_redo();
            }
        });

        ui.separator();
        ui.label(format!("Pași Undo disponibili: {undo_len} / 30"));
        ui.label(format!("Pași Redo disponibili: {redo_len}"));
    }
}