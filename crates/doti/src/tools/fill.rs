use eframe::egui;

#[derive(Clone, Debug)]
pub(crate) struct FillProps {
    pub(crate) tolerance: u8,
    pub(crate) contiguous: bool,
}

impl Default for FillProps {
    fn default() -> Self {
        Self {
            tolerance: 0,
            contiguous: true,
        }
    }
}

impl FillProps {
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Galeata (Flood Fill)");
        ui.separator();

        ui.add(egui::Slider::new(&mut self.tolerance, 0..=255).text("Toleranta"));
        ui.checkbox(&mut self.contiguous, "Doar zona conectata (Contiguous)")
            .on_hover_text(
                "Bifat: umple doar pixelii conectati. Debifat: inlocuieste peste tot in frame.",
            );
    }
}