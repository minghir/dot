use crate::model::{ColorDepth, STANDARD_SIZES};
use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_frames_panel(&mut self, ui: &mut egui::Ui) {
        if ui
            .button("Genereaza set Windows (32/8/4-bit)")
            .on_hover_text(
                "Creeaza automat 256x256 (32-bit) + 48/32/16 in variantele 32-bit, 8-bit si 4-bit",
            )
            .clicked()
        {
            self.generate_full_windows_set();
        }

        ui.separator();
        ui.label("Adauga / converteste format:");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("combo_size")
                .width(68.0)
                .selected_text(format!("{}x{}", self.new_frame_size, self.new_frame_size))
                .show_ui(ui, |ui| {
                    for &s in &STANDARD_SIZES {
                        ui.selectable_value(&mut self.new_frame_size, s, format!("{s}x{s}"));
                    }
                });

            egui::ComboBox::from_id_salt("combo_depth")
                .width(75.0)
                .selected_text(self.new_frame_depth.short_label())
                .show_ui(ui, |ui| {
                    for &d in &ColorDepth::ALL {
                        ui.selectable_value(&mut self.new_frame_depth, d, d.full_label());
                    }
                });

            if ui.button("+").clicked() {
                self.add_or_replace_variant(self.new_frame_size, self.new_frame_depth);
            }
        });

        ui.separator();
        let mut clicked_frame = None;
        let mut remove_frame = None;
        let can_remove = self.frames.len() > 1;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, frame) in self.frames.iter().enumerate() {
                let selected = idx == self.active_frame;
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        let label = format!(
                            "{}x{} · {}",
                            frame.width,
                            frame.height,
                            frame.color_depth.short_label()
                        );
                        if ui.selectable_label(selected, label).clicked() {
                            clicked_frame = Some(idx);
                        }
                        if let Some(tex) = &frame.texture {
                            let preview_size = (frame.width as f32).clamp(16.0, 48.0);
                            ui.image((tex.id(), egui::vec2(preview_size, preview_size)));
                        }
                        if can_remove && ui.small_button("X").clicked() {
                            remove_frame = Some(idx);
                        }
                    });
                });
            }
        });

        if let Some(idx) = clicked_frame {
            self.active_frame = idx;
            self.auto_fit_zoom();
        }
        if let Some(idx) = remove_frame {
            self.frames.remove(idx);
            if self.active_frame >= self.frames.len() {
                self.active_frame = self.frames.len() - 1;
            }
            self.auto_fit_zoom();
        }
    }
}