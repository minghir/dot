use crate::model::{ColorDepth, Tool};
use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_tool_properties_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| match self.tool {
            Tool::Pencil => self.tool_settings.pencil.ui(ui),
            Tool::Eraser => self.tool_settings.eraser.ui(ui),
            Tool::Fill => self.tool_settings.fill.ui(ui),
            Tool::Eyedropper => self.tool_settings.eyedropper.ui(ui),
            Tool::SelectRect => self.show_select_rect_properties(ui),
        });
    }

    fn show_select_rect_properties(&mut self, ui: &mut egui::Ui) {
        ui.strong("Selectie Dreptunghi");
        ui.separator();

        ui.checkbox(
            &mut self.tool_settings.select_rect.constrain_square,
            "Forteaza patrat (1:1)",
        );
        ui.separator();

        if let Some((x0, y0, x1, y1)) = self.selection {
            let w = x1 - x0 + 1;
            let h = y1 - y0 + 1;
            ui.monospace(format!("Pozitie: ({x0}, {y0})"));
            ui.monospace(format!("Dimensiune: {w} x {h} px"));
            ui.separator();

            ui.horizontal_wrapped(|ui| {
                if ui.button("Umple selectia").clicked() {
                    let col = self.primary_color;
                    let dithering = self.use_dithering;
                    let frame = &mut self.frames[self.active_frame];
                    frame.push_undo();
                    for y in y0..=y1 {
                        for x in x0..=x1 {
                            frame.set_pixel(x, y, col);
                        }
                    }
                    if frame.color_depth != ColorDepth::Bit32 {
                        frame.apply_color_depth(dithering);
                    }
                    self.status_msg = format!("Umplut selectia {}x{}.", w, h);
                }

                if ui.button("Sterge (Del)").clicked() {
                    self.clear_selection_pixels();
                }

                if ui.button("Deselect (Esc)").clicked() {
                    self.deselect();
                }
            });
        } else {
            ui.label("Nicio selectie activa. Trage pe canvas pentru a selecta.");
            if ui.button("Select All (Ctrl+A)").clicked() {
                self.select_all();
            }
        }
    }
}