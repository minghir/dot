use crate::model::Tool;
use crate::IconEditorApp;
use eframe::egui;

fn draw_pencil_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let tip = egui::pos2(rect.min.x + 3.0_f32, rect.max.y - 3.0_f32);
    let top = egui::pos2(rect.max.x - 3.0_f32, rect.min.y + 3.0_f32);
    painter.line_segment([tip, top], stroke);

    let mid = egui::pos2(rect.min.x + 5.5_f32, rect.max.y - 5.5_f32);
    let side = egui::pos2(rect.min.x + 2.0_f32, rect.max.y - 6.5_f32);
    painter.line_segment([tip, side], stroke);
    painter.line_segment([mid, side], stroke);
}

fn draw_eraser_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let body = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 3.0_f32, rect.min.y + 5.0_f32),
        egui::pos2(rect.max.x - 3.0_f32, rect.max.y - 3.0_f32),
    );
    painter.rect_stroke(body, 2.0_f32, stroke);
    painter.line_segment(
        [
            egui::pos2(rect.min.x + 7.5_f32, rect.min.y + 5.0_f32),
            egui::pos2(rect.min.x + 11.0_f32, rect.max.y - 3.0_f32),
        ],
        stroke,
    );
}

fn draw_fill_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let p1 = egui::pos2(rect.min.x + 8.0_f32, rect.min.y + 3.0_f32);
    let p2 = egui::pos2(rect.max.x - 3.0_f32, rect.min.y + 8.0_f32);
    let p3 = egui::pos2(rect.min.x + 8.0_f32, rect.max.y - 3.0_f32);
    let p4 = egui::pos2(rect.min.x + 3.0_f32, rect.min.y + 8.0_f32);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p1], stroke);

    painter.circle_filled(
        egui::pos2(rect.max.x - 2.5_f32, rect.max.y - 4.0_f32),
        1.8_f32,
        color,
    );
}

fn draw_eyedropper_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    let tip = egui::pos2(rect.min.x + 3.0_f32, rect.max.y - 3.0_f32);
    let neck = egui::pos2(rect.min.x + 9.5_f32, rect.min.y + 6.5_f32);
    let head = egui::pos2(rect.max.x - 3.0_f32, rect.min.y + 3.0_f32);
    painter.line_segment([tip, neck], stroke);
    painter.circle_stroke(head, 2.2_f32, stroke);
}

fn draw_select_rect_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let sel_box = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 2.5_f32, rect.min.y + 2.5_f32),
        egui::pos2(rect.max.x - 2.5_f32, rect.max.y - 2.5_f32),
    );
    let stroke = egui::Stroke::new(1.4_f32, color);
    painter.rect_stroke(sel_box, 0.0_f32, stroke);
}

fn tool_button(
    ui: &mut egui::Ui,
    selected: bool,
    hover_text: &str,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, egui::Color32),
) -> bool {
    let desired_size = egui::vec2(28.0_f32, 28.0_f32);
    let (rect, resp) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if resp.hovered() || selected {
        let bg = if selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.hovered.bg_fill
        };
        ui.painter().rect_filled(rect, 4.0_f32, bg);
    }

    let fg = if selected {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };

    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(16.0_f32, 16.0_f32));
    draw_icon(ui.painter(), icon_rect, fg);

    let resp = if !hover_text.is_empty() {
        resp.on_hover_text(hover_text)
    } else {
        resp
    };

    resp.clicked()
}

impl IconEditorApp {
    pub(crate) fn show_tools_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if tool_button(ui, self.tool == Tool::Pencil, "Creion (Desen)", draw_pencil_icon) {
                    self.tool = Tool::Pencil;
                }
                if tool_button(
                    ui,
                    self.tool == Tool::Eraser,
                    "Radiera (Transparent)",
                    draw_eraser_icon,
                ) {
                    self.tool = Tool::Eraser;
                }
                if tool_button(ui, self.tool == Tool::Fill, "Galeata (Flood Fill)", draw_fill_icon) {
                    self.tool = Tool::Fill;
                }
                if tool_button(
                    ui,
                    self.tool == Tool::Eyedropper,
                    "Pipeta (Alege culoare)",
                    draw_eyedropper_icon,
                ) {
                    self.tool = Tool::Eyedropper;
                }
                if tool_button(
                    ui,
                    self.tool == Tool::SelectRect,
                    "Selectie Dreptunghi",
                    draw_select_rect_icon,
                ) {
                    self.tool = Tool::SelectRect;
                }
            });

            ui.separator();
            ui.strong("Actiuni Frame");
            if ui.button("Sterge fundal (Colturi -> Transparent)").clicked() {
                let f = &mut self.frames[self.active_frame];
                f.push_undo();
                f.remove_corner_background(35.0);
            }
            if ui.button("Sterge Tot Frame-ul").clicked() {
                let f = &mut self.frames[self.active_frame];
                f.push_undo();
                f.pixels.fill([0, 0, 0, 0]);
                f.dirty = true;
            }
            if ui.button("Propaga la toate formatele").clicked() {
                self.regenerate_from_current();
            }

            ui.separator();
            ui.strong("Cuantizare & Afisare");
            ui.checkbox(&mut self.use_dithering, "Dithering (8/4/1-bit)");
            if ui.button("Recalculeaza paleta frame").clicked() {
                let dithering = self.use_dithering;
                let f = &mut self.frames[self.active_frame];
                f.push_undo();
                f.apply_color_depth(dithering);
            }

            ui.separator();
            ui.checkbox(&mut self.show_grid, "Afiseaza Grid");
            ui.add(egui::Slider::new(&mut self.zoom, 2.0..=32.0).text("Zoom"));
        });
    }
}