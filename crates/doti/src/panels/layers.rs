use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_layers_panel(&mut self, ui: &mut egui::Ui) {
        let active = &self.frames[self.active_frame];
        ui.horizontal(|ui| {
            ui.strong(format!("Layers ({}x{})", active.width, active.height));
        });
        ui.separator();

        // Placeholder curat până la conectarea vectorului `layers: Vec<Layer>` pe IconFrame
        ui.group(|ui| {
            ui.horizontal(|ui| {
                let mut visible = true;
                ui.checkbox(&mut visible, "");
                let _ = ui.selectable_label(true, "Layer 0 (Base)");
            });
        });

        ui.add_space(4.0);
        ui.label("Următorul pas: multi-layer cu opacitate, vizibilitate și blend RGBA.");
    }
}