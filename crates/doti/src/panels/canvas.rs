use crate::model::{plot_line, Tool};
use crate::tools::{BrushParams, BrushShape};
use crate::IconEditorApp;
use eframe::egui;

fn apply_pixel_perfect_filter(stroke: &mut Vec<(i32, i32)>) {
    if stroke.len() < 3 {
        return;
    }
    let mut filtered = Vec::with_capacity(stroke.len());
    filtered.push(stroke[0]);
    let mut i = 1;
    while i + 1 < stroke.len() {
        let prev = *filtered.last().unwrap();
        let curr = stroke[i];
        let next = stroke[i + 1];

        let is_l_corner = (prev.0 == curr.0 || prev.1 == curr.1)
            && (curr.0 == next.0 || curr.1 == next.1)
            && (prev.0 - next.0).abs() == 1
            && (prev.1 - next.1).abs() == 1;

        if is_l_corner {
            i += 1;
            continue;
        }
        filtered.push(curr);
        i += 1;
    }
    filtered.push(*stroke.last().unwrap());
    *stroke = filtered;
}

impl IconEditorApp {
    fn stamp_brush_on_frame(
        frame: &mut crate::model::IconFrame,
        cx: i32,
        cy: i32,
        color: [u8; 4],
        params: BrushParams,
    ) {
        let w = frame.width as i32;
        let h = frame.height as i32;
        let size = params.size.max(1) as i32;
        let half_lo = (size - 1) / 2;
        let half_hi = size / 2;
        let radius_sq = (size as f32 * 0.5) * (size as f32 * 0.5);

        let mut draw_at = |bx: i32, by: i32| {
            for dy in -half_lo..=half_hi {
                for dx in -half_lo..=half_hi {
                    if params.shape == BrushShape::Circle && size > 2 {
                        let dist_sq = (dx as f32) * (dx as f32) + (dy as f32) * (dy as f32);
                        if dist_sq > radius_sq {
                            continue;
                        }
                    }
                    let px = bx + dx;
                    let py = by + dy;
                    if px >= 0 && px < w && py >= 0 && py < h {
                        frame.set_pixel_raw(px as u32, py as u32, color);
                    }
                }
            }
        };

        draw_at(cx, cy);
        if params.mirror_x {
            draw_at(w - 1 - cx, cy);
        }
        if params.mirror_y {
            draw_at(cx, h - 1 - cy);
        }
        if params.mirror_x && params.mirror_y {
            draw_at(w - 1 - cx, h - 1 - cy);
        }

        frame.recompose();
    }

    pub(crate) fn show_canvas_panel(&mut self, ui: &mut egui::Ui) {
        let checker_tex_id = self.get_checker_texture(ui.ctx());

        let scroll_delta_y = ui.ctx().input(|i| {
            if i.modifiers.ctrl {
                i.smooth_scroll_delta.y
            } else {
                0.0
            }
        });
        if scroll_delta_y.abs() > 0.0 && ui.ui_contains_pointer() {
            let factor = if scroll_delta_y > 0.0 { 1.12 } else { 0.89 };
            self.zoom = (self.zoom * factor).clamp(2.0, 32.0);
        }

        egui::ScrollArea::both()
            .drag_to_scroll(false)
            .show(ui, |ui| {
                let frame = &mut self.frames[self.active_frame];
                let canvas_w = frame.width as f32 * self.zoom;
                let canvas_h = frame.height as f32 * self.zoom;

                let avail = ui.available_size();
                let pad_x = ((avail.x - canvas_w) * 0.5).max(8.0);
                let pad_y = ((avail.y - canvas_h) * 0.5).max(8.0);

                let (_, outer_rect) = ui.allocate_exact_size(
                    egui::vec2(canvas_w + pad_x * 2.0, canvas_h + pad_y * 2.0),
                    egui::Sense::hover(),
                );
                let canvas_rect = egui::Rect::from_min_size(
                    egui::pos2(outer_rect.rect.min.x + pad_x, outer_rect.rect.min.y + pad_y),
                    egui::vec2(canvas_w, canvas_h),
                );

                let response = ui.interact(
                    canvas_rect,
                    ui.id().with("dot_pixel_canvas"),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter_at(canvas_rect);

                let uv_checker = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(frame.width as f32 / 2.0, frame.height as f32 / 2.0),
                );
                painter.image(checker_tex_id, canvas_rect, uv_checker, egui::Color32::WHITE);

                if let Some(tex) = &frame.texture {
                    painter.image(
                        tex.id(),
                        canvas_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                if self.show_grid && self.zoom >= 6.0 {
                    let grid_stroke = egui::Stroke::new(0.5_f32, egui::Color32::from_gray(75));
                    let clip = painter.clip_rect();

                    let start_x = (((clip.min.x - canvas_rect.min.x) / self.zoom).floor() as i32)
                        .max(0) as u32;
                    let end_x = (((clip.max.x - canvas_rect.min.x) / self.zoom).ceil() as i32)
                        .min(frame.width as i32)
                        .max(0) as u32;

                    let start_y = (((clip.min.y - canvas_rect.min.y) / self.zoom).floor() as i32)
                        .max(0) as u32;
                    let end_y = (((clip.max.y - canvas_rect.min.y) / self.zoom).ceil() as i32)
                        .min(frame.height as i32)
                        .max(0) as u32;

                    let y_min = clip.min.y.max(canvas_rect.min.y);
                    let y_max = clip.max.y.min(canvas_rect.max.y);
                    for x in start_x..=end_x {
                        let px = canvas_rect.min.x + x as f32 * self.zoom;
                        painter.line_segment(
                            [egui::pos2(px, y_min), egui::pos2(px, y_max)],
                            grid_stroke,
                        );
                    }

                    let x_min = clip.min.x.max(canvas_rect.min.x);
                    let x_max = clip.max.x.min(canvas_rect.max.x);
                    for y in start_y..=end_y {
                        let py = canvas_rect.min.y + y as f32 * self.zoom;
                        painter.line_segment(
                            [egui::pos2(x_min, py), egui::pos2(x_max, py)],
                            grid_stroke,
                        );
                    }
                }

                painter.rect_stroke(
                    canvas_rect.expand(1.0_f32),
                    0.0,
                    egui::Stroke::new(1.0_f32, egui::Color32::from_gray(95)),
                );

                if let Some((x0, y0, x1, y1)) = self.selection {
                    let sel_rect = egui::Rect::from_min_max(
                        egui::pos2(
                            canvas_rect.min.x + x0 as f32 * self.zoom,
                            canvas_rect.min.y + y0 as f32 * self.zoom,
                        ),
                        egui::pos2(
                            canvas_rect.min.x + (x1 + 1) as f32 * self.zoom,
                            canvas_rect.min.y + (y1 + 1) as f32 * self.zoom,
                        ),
                    );
                    painter.rect_filled(
                        sel_rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14),
                    );
                    painter.rect_stroke(
                        sel_rect,
                        0.0,
                        egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 190),
                        ),
                    );
                    painter.rect_stroke(
                        sel_rect.expand(1.0_f32),
                        0.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 59, 48)),
                    );
                }

                let pointer_pos = response
                    .interact_pointer_pos()
                    .or_else(|| response.hover_pos());

                self.hovered_coord = None;

                if let Some(pos) = pointer_pos {
                    let gx = ((pos.x - canvas_rect.min.x) / self.zoom).floor() as i32;
                    let gy = ((pos.y - canvas_rect.min.y) / self.zoom).floor() as i32;

                    if gx >= 0
                        && gx < frame.width as i32
                        && gy >= 0
                        && gy < frame.height as i32
                    {
                        self.hovered_coord = Some((gx as u32, gy as u32));

                        let brush_sz = match self.tool {
                            Tool::Pencil => self.tool_settings.pencil.size.max(1) as i32,
                            Tool::Eraser => self.tool_settings.eraser.size.max(1) as i32,
                            _ => 1,
                        };
                        let half_lo = (brush_sz - 1) / 2;
                        let px_rect = egui::Rect::from_min_size(
                            egui::pos2(
                                canvas_rect.min.x + (gx - half_lo) as f32 * self.zoom,
                                canvas_rect.min.y + (gy - half_lo) as f32 * self.zoom,
                            ),
                            egui::vec2(self.zoom * brush_sz as f32, self.zoom * brush_sz as f32),
                        );
                        painter.rect_stroke(
                            px_rect,
                            0.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
                        );

                        if response.drag_started() {
                            if matches!(self.tool, Tool::Pencil | Tool::Eraser) {
                                frame.push_undo();
                                self.last_draw_pos = Some((gx, gy));
                                self.stroke_points.clear();
                                self.stroke_points.push((gx, gy));
                            } else if self.tool == Tool::SelectRect {
                                self.select_drag_start = Some((gx, gy));
                            }
                        }

                        if response.clicked() || response.dragged() {
                            match self.tool {
                                Tool::SelectRect => {
                                    if self.select_drag_start.is_none() {
                                        self.select_drag_start = Some((gx, gy));
                                    }
                                    let start = self.select_drag_start.unwrap_or((gx, gy));
                                    let (mut end_x, mut end_y) = (gx, gy);

                                    if self.tool_settings.select_rect.constrain_square {
                                        let dx = end_x - start.0;
                                        let dy = end_y - start.1;
                                        let side = dx.abs().max(dy.abs());
                                        end_x = start.0 + side * dx.signum();
                                        end_y = start.1 + side * dy.signum();
                                    }

                                    let min_x =
                                        start.0.min(end_x).clamp(0, frame.width as i32 - 1) as u32;
                                    let min_y =
                                        start.1.min(end_y).clamp(0, frame.height as i32 - 1) as u32;
                                    let max_x =
                                        start.0.max(end_x).clamp(0, frame.width as i32 - 1) as u32;
                                    let max_y =
                                        start.1.max(end_y).clamp(0, frame.height as i32 - 1) as u32;
                                    self.selection = Some((min_x, min_y, max_x, max_y));
                                }
                                Tool::Pencil | Tool::Eraser => {
                                    let (draw_col, params) = if self.tool == Tool::Pencil {
                                        (
                                            self.primary_color,
                                            self.tool_settings.pencil.brush_params(),
                                        )
                                    } else {
                                        ([0, 0, 0, 0], self.tool_settings.eraser.brush_params())
                                    };

                                    if response.clicked() && !response.dragged() {
                                        frame.push_undo();
                                    }

                                    let (sx, sy) = self.last_draw_pos.unwrap_or((gx, gy));
                                    let mut segment = Vec::new();
                                    plot_line(sx, sy, gx, gy, |px, py| {
                                        segment.push((px, py));
                                    });

                                    if self.tool == Tool::Pencil
                                        && self.tool_settings.pencil.pixel_perfect
                                        && self.tool_settings.pencil.size == 1
                                    {
                                        for &pt in &segment {
                                            if self.stroke_points.last() != Some(&pt) {
                                                self.stroke_points.push(pt);
                                            }
                                        }
                                        let before = self.stroke_points.len();
                                        apply_pixel_perfect_filter(&mut self.stroke_points);
                                        if self.stroke_points.len() < before {
                                            if let Some(base) = frame.undo_stack.last().cloned() {
                                                frame.layers = base.layers;
                                                frame.recompose();
                                            }
                                            for &(px, py) in &self.stroke_points {
                                                Self::stamp_brush_on_frame(
                                                    frame, px, py, draw_col, params,
                                                );
                                            }
                                        } else {
                                            for &(px, py) in &segment {
                                                Self::stamp_brush_on_frame(
                                                    frame, px, py, draw_col, params,
                                                );
                                            }
                                        }
                                    } else {
                                        for &(px, py) in &segment {
                                            Self::stamp_brush_on_frame(
                                                frame, px, py, draw_col, params,
                                            );
                                        }
                                    }

                                    self.last_draw_pos = Some((gx, gy));
                                }
                                Tool::Fill => {
                                    if response.clicked() {
                                        frame.push_undo();
                                        frame.flood_fill_with_options(
                                            gx as u32,
                                            gy as u32,
                                            self.primary_color,
                                            self.tool_settings.fill.tolerance,
                                            self.tool_settings.fill.contiguous,
                                        );
                                    }
                                }
                                Tool::Eyedropper => {
                                    self.primary_color =
                                        if self.tool_settings.eyedropper.sample_all_layers {
                                            frame.get_pixel(gx as u32, gy as u32)
                                        } else {
                                            frame.get_active_layer_pixel(gx as u32, gy as u32)
                                        };
                                    if self.tool_settings.eyedropper.auto_switch_to_pencil {
                                        self.tool = Tool::Pencil;
                                    }
                                }
                            }
                        }
                    }
                }

                if response.drag_stopped() {
                    self.last_draw_pos = None;
                    self.select_drag_start = None;
                    self.stroke_points.clear();
                }
            });
    }
}