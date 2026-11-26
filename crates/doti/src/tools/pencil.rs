use super::{BrushParams, BrushShape};
use eframe::egui;

#[derive(Clone, Debug)]
pub(crate) struct PencilProps {
    pub(crate) size: u32,
    pub(crate) shape: BrushShape,
    pub(crate) pixel_perfect: bool,
    pub(crate) mirror_x: bool,
    pub(crate) mirror_y: bool,
}

impl Default for PencilProps {
    fn default() -> Self {
        Self {
            size: 1,
            shape: BrushShape::Square,
            pixel_perfect: false,
            mirror_x: false,
            mirror_y: false,
        }
    }
}

impl PencilProps {
    pub(crate) fn brush_params(&self) -> BrushParams {
        BrushParams {
            size: self.size,
            shape: self.shape,
            mirror_x: self.mirror_x,
            mirror_y: self.mirror_y,
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Creion (Pencil)");
        ui.separator();

        ui.add(egui::Slider::new(&mut self.size, 1..=16).text("Dimensiune (px)"));

        ui.horizontal(|ui| {
            ui.label("Forma:");
            ui.selectable_value(&mut self.shape, BrushShape::Square, "Patrat");
            ui.selectable_value(&mut self.shape, BrushShape::Circle, "Cerc");
        });

        ui.add_enabled(
            self.size == 1,
            egui::Checkbox::new(&mut self.pixel_perfect, "Pixel Perfect (1px)"),
        )
        .on_hover_text("Elimina colturile duble in scara pe diagonale (activ doar la 1px)");

        ui.separator();
        ui.strong("Simetrie (Mirror)");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.mirror_x, "Orizontal (X)");
            ui.checkbox(&mut self.mirror_y, "Vertical (Y)");
        });
    }
}