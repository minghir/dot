use eframe::egui;

#[derive(Clone, Debug)]
pub(crate) struct EyedropperProps {
    pub(crate) auto_switch_to_pencil: bool,
    pub(crate) sample_all_layers: bool,
}

impl Default for EyedropperProps {
    fn default() -> Self {
        Self {
            auto_switch_to_pencil: true,
            sample_all_layers: true,
        }
    }
}

impl EyedropperProps {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Pipeta (Eyedropper)");
        ui.separator();

        ui.checkbox(
            &mut self.auto_switch_to_pencil,
            "Revino automat la Creion dupa selectie",
        );
        ui.checkbox(
            &mut self.sample_all_layers,
            "Esantioneaza din toate layerele vizibile",
        );
    }
}