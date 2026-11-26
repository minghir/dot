use crate::dock::{create_default_dock_state, PanelTab};
use crate::model::Tool;
use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn show_top_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    let preserved_dock = std::mem::replace(
                        &mut self.dock_state,
                        egui_dock::DockState::new(vec![]),
                    );
                    *self = Self::default();
                    self.dock_state = preserved_dock;
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Open").shortcut_text("Ctrl+O")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Windows Icon", &["ico"])
                        .pick_file()
                    {
                        self.open_ico(&path);
                    }
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                    self.save_current_ico();
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Save As").shortcut_text("Ctrl+Shift+S"))
                    .clicked()
                {
                    self.save_ico_as();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Close").shortcut_text("Ctrl+W")).clicked() {
                    self.close_file();
                    ui.close_menu();
                }

                ui.separator();

                ui.menu_button("Import", |ui| {
                    if ui
                        .add(egui::Button::new("Image (PNG / JPG / BMP)...").shortcut_text("Ctrl+I"))
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Imagini", &["png", "jpg", "jpeg", "bmp"])
                            .pick_file()
                        {
                            self.import_png(&path);
                        }
                        ui.close_menu();
                    }
                });

                ui.menu_button("Export", |ui| {
                    if ui
                        .add(egui::Button::new("Active Frame as PNG...").shortcut_text("Ctrl+E"))
                        .clicked()
                    {
                        self.export_active_png();
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Windows ICO Set (32/8/4-bit)...")
                                .shortcut_text("Ctrl+Shift+E"),
                        )
                        .clicked()
                    {
                        self.generate_full_windows_set();
                        self.save_ico_as();
                        ui.close_menu();
                    }
                });

                ui.separator();

                if ui.add(egui::Button::new("Exit")).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close_menu();
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui.add(egui::Button::new("Undo").shortcut_text("Ctrl+Z")).clicked() {
                    self.edit_undo();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Redo").shortcut_text("Ctrl+Y")).clicked() {
                    self.edit_redo();
                    ui.close_menu();
                }

                ui.separator();

                if ui.add(egui::Button::new("Cut").shortcut_text("Ctrl+X")).clicked() {
                    self.edit_cut();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Copy").shortcut_text("Ctrl+C")).clicked() {
                    self.edit_copy();
                    ui.close_menu();
                }
                if ui.add(egui::Button::new("Paste").shortcut_text("Ctrl+V")).clicked() {
                    self.edit_paste();
                    ui.close_menu();
                }
            });

            ui.menu_button("Select", |ui| {
                let is_rect = self.tool == Tool::SelectRect;
                if ui.selectable_label(is_rect, "Rectangle").clicked() {
                    self.tool = Tool::SelectRect;
                    self.status_msg = "Unealta activa: Rectangle Select. Trage pe grid.".to_string();
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .add(egui::Button::new("Select All").shortcut_text("Ctrl+A"))
                    .clicked()
                {
                    self.select_all();
                    ui.close_menu();
                }
                if ui
                    .add(egui::Button::new("Deselect").shortcut_text("Esc"))
                    .clicked()
                {
                    self.deselect();
                    ui.close_menu();
                }

                ui.separator();

                let has_sel = self.selection.is_some();
                if ui
                    .add_enabled(
                        has_sel,
                        egui::Button::new("Clear Selection").shortcut_text("Del"),
                    )
                    .clicked()
                {
                    self.clear_selection_pixels();
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                for &tab in PanelTab::ALL {
                    if !tab.is_closeable() {
                        continue;
                    }

                    let mut is_open = self.dock_state.find_tab(&tab).is_some();
                    if ui.checkbox(&mut is_open, tab.title()).clicked() {
                        if let Some((surface, node, tab_idx)) = self.dock_state.find_tab(&tab) {
                            self.dock_state[surface].remove_tab((node, tab_idx));
                        } else {
                            self.dock_state.push_to_focused_leaf(tab);
                        }
                        ui.close_menu();
                    }
                }

                ui.separator();

                ui.checkbox(&mut self.show_grid, "Afiseaza Grid");
                if ui.button("Reset Layout").clicked() {
                    self.dock_state = create_default_dock_state();
                    ui.close_menu();
                }
            });
        });
    }

    pub(crate) fn show_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let active = &self.frames[self.active_frame];
            ui.strong(format!(
                "[{}x{} · {}]",
                active.width,
                active.height,
                active.color_depth.full_label()
            ));
            ui.separator();
            ui.label(&self.status_msg);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((x, y)) = self.hovered_coord {
                    ui.monospace(format!("X: {x}, Y: {y}"));
                } else {
                    ui.monospace("X: —, Y: —");
                }
            });
        });
    }
}