use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_layers_panel(&mut self, ui: &mut egui::Ui) {
        let frame = &mut self.frames[self.active_frame];
        let active_idx = frame.active_layer.min(frame.layers.len().saturating_sub(1));

        // Bara de acțiuni sus
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("+ Layer")
                .on_hover_text("Adauga un layer nou deasupra celui activ")
                .clicked()
            {
                frame.add_layer(None);
            }

            if ui
                .button("Dup")
                .on_hover_text("Duplica layer-ul activ")
                .clicked()
            {
                frame.duplicate_layer(active_idx);
            }

            if ui
                .add_enabled(active_idx > 0, egui::Button::new("Merge Down"))
                .on_hover_text("Combina layer-ul activ cu cel de dedesubt")
                .clicked()
            {
                frame.merge_down(active_idx);
            }

            if ui
                .add_enabled(frame.layers.len() > 1, egui::Button::new("Flatten"))
                .on_hover_text("Uneste toate layerele intr-unul singur")
                .clicked()
            {
                frame.flatten_layers();
            }
        });

        ui.separator();

        // Slider opacitate pentru layer-ul activ
        let mut opacity_changed = false;
        let mut push_undo_opacity = false;
        {
            let active_layer = &mut frame.layers[active_idx];
            ui.horizontal(|ui| {
                ui.label("Opacitate:");
                let slider = ui.add(
                    egui::Slider::new(&mut active_layer.opacity, 0.0..=1.0)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                );
                if slider.drag_started() {
                    push_undo_opacity = true;
                }
                if slider.changed() {
                    opacity_changed = true;
                }
            });
        }
        if push_undo_opacity {
            frame.push_undo();
        }
        if opacity_changed {
            frame.recompose();
        }

        ui.separator();

        // Lista de layere (randată de sus în jos: ultimul index este deasupra)
        let mut select_idx = None;
        let mut move_op: Option<(usize, usize)> = None;
        let mut delete_idx = None;
        let mut visibility_toggled = false;

        let layer_count = frame.layers.len();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for idx in (0..layer_count).rev() {
                let is_active = idx == active_idx;
                let frame_bg = if is_active {
                    ui.visuals().selection.bg_fill.linear_multiply(0.45)
                } else {
                    egui::Color32::TRANSPARENT
                };

                egui::Frame::none()
                    .fill(frame_bg)
                    .inner_margin(egui::Margin::symmetric(4.0, 3.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // 1. Vizibilitate
                            if ui.checkbox(&mut frame.layers[idx].visible, "").changed() {
                                visibility_toggled = true;
                            }

                            // 2. Nume layer (click selectează, dublu-click editează)
                            if self.renaming_layer == Some(idx) {
                                let edit = ui.add(
                                    egui::TextEdit::singleline(&mut frame.layers[idx].name)
                                        .desired_width(110.0),
                                );
                                if edit.lost_focus()
                                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    self.renaming_layer = None;
                                } else {
                                    edit.request_focus();
                                }
                            } else {
                                let label_text = if frame.layers[idx].opacity < 0.999 {
                                    format!(
                                        "{} ({:.0}%)",
                                        frame.layers[idx].name,
                                        frame.layers[idx].opacity * 100.0
                                    )
                                } else {
                                    frame.layers[idx].name.clone()
                                };

                                let resp = ui.selectable_label(is_active, label_text);
                                if resp.clicked() {
                                    select_idx = Some(idx);
                                }
                                if resp.double_clicked() {
                                    select_idx = Some(idx);
                                    self.renaming_layer = Some(idx);
                                }
                            }

                            // 3. Butoane ordine & ștergere aliniate în dreapta
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if layer_count > 1 && ui.small_button("X").clicked() {
                                        delete_idx = Some(idx);
                                    }
                                    if idx > 0 && ui.small_button("v").clicked() {
                                        move_op = Some((idx, idx - 1));
                                    }
                                    if idx + 1 < layer_count && ui.small_button("^").clicked() {
                                        move_op = Some((idx, idx + 1));
                                    }
                                },
                            );
                        });
                    });
            }
        });

        if visibility_toggled {
            frame.recompose();
        }
        if let Some(idx) = select_idx {
            frame.active_layer = idx;
        }
        if let Some((from, to)) = move_op {
            frame.move_layer(from, to);
        }
        if let Some(idx) = delete_idx {
            frame.remove_layer(idx);
            if self.renaming_layer == Some(idx) {
                self.renaming_layer = None;
            }
        }
    }
}