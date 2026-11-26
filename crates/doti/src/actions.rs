use crate::model::{ColorDepth, IconFrame, STANDARD_SIZES};
use crate::IconEditorApp;
use eframe::egui;
use ico::IconDir;
use image::imageops::FilterType;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

impl IconEditorApp {
    pub(crate) fn from_startup_path(path: Option<PathBuf>) -> IconEditorApp {
        let mut app = IconEditorApp::default();
        if let Some(p) = path {
            if p.is_file() {
                match p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .as_deref()
                {
                    Some("ico") => {
                        app.current_file = Some(p.clone());
                        app.open_ico(&p);
                    }
                    Some("png" | "jpg" | "jpeg" | "bmp") => {
                        app.import_png(&p);
                    }
                    _ => app.status_msg = format!("Format nesuportat la pornire: {:?}", p),
                }
            } else if p.is_dir() {
                app.status_msg = format!("Director proiect: {:?}", p);
            }
        }
        app
    }

    pub(crate) fn save_current_ico(&mut self) {
        if let Some(path) = self.current_file.clone() {
            self.save_ico(&path);
        } else {
            self.save_ico_as();
        }
    }

    pub(crate) fn save_ico_as(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .set_file_name("icon.ico")
            .add_filter("Windows Icon", &["ico"]);

        if let Some(existing) = &self.current_file {
            if let Some(dir) = existing.parent() {
                dialog = dialog.set_directory(dir);
            }
            if let Some(name) = existing.file_name().and_then(|n| n.to_str()) {
                dialog = dialog.set_file_name(name);
            }
        }

        if let Some(path) = dialog.save_file() {
            self.save_ico(&path);
        }
    }

    pub(crate) fn close_file(&mut self) {
        let preserved_dock = std::mem::replace(
            &mut self.dock_state,
            egui_dock::DockState::new(vec![]),
        );
        *self = Self::default();
        self.dock_state = preserved_dock;
        self.status_msg = "Fisier inchis.".to_string();
    }

    pub(crate) fn export_active_png(&mut self) {
        let frame = &self.frames[self.active_frame];
        let default_name = format!("icon_{}x{}.png", frame.width, frame.height);
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("PNG Image", &["png"])
            .save_file()
        else {
            return;
        };

        let Some(img) =
            image::RgbaImage::from_raw(frame.width, frame.height, frame.to_flat_rgba())
        else {
            self.status_msg = "Eroare la pregatirea imaginii PNG.".to_string();
            return;
        };

        match img.save(&path) {
            Ok(_) => {
                self.status_msg = format!(
                    "Exportat PNG: {:?}",
                    path.file_name().unwrap_or_default()
                );
            }
            Err(e) => {
                self.status_msg = format!("Eroare export PNG: {e}");
            }
        }
    }

    pub(crate) fn edit_undo(&mut self) {
        self.frames[self.active_frame].undo();
        self.status_msg = "Undo aplicat.".to_string();
    }

    pub(crate) fn edit_redo(&mut self) {
        self.frames[self.active_frame].redo();
        self.status_msg = "Redo aplicat.".to_string();
    }

    pub(crate) fn edit_copy(&mut self) {
        let frame = &self.frames[self.active_frame];

        if let Some((x0, y0, x1, y1)) = self.selection {
            let w = x1 - x0 + 1;
            let h = y1 - y0 + 1;
            let mut region = Vec::with_capacity((w * h) as usize);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    region.push(frame.get_pixel(x, y));
                }
            }
            self.clipboard = Some((w, h, region));
            self.status_msg = format!("Copiat selectia {}x{}.", w, h);
        } else {
            self.clipboard = Some((frame.width, frame.height, frame.pixels.clone()));
            self.status_msg = format!(
                "Copiat tot frame-ul {}x{} ({}).",
                frame.width,
                frame.height,
                frame.color_depth.short_label()
            );
        }
    }

    pub(crate) fn edit_cut(&mut self) {
        self.edit_copy();

        let dithering = self.use_dithering;
        let sel = self.selection;
        let frame = &mut self.frames[self.active_frame];
        frame.push_undo();

        if let Some((x0, y0, x1, y1)) = sel {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    frame.set_pixel(x, y, [0, 0, 0, 0]);
                }
            }
            if frame.color_depth != ColorDepth::Bit32 {
                frame.apply_color_depth(dithering);
            }
            frame.dirty = true;
            self.status_msg = format!("Decupat selectia {}x{}.", x1 - x0 + 1, y1 - y0 + 1);
        } else {
            frame.pixels.fill([0, 0, 0, 0]);
            if frame.color_depth != ColorDepth::Bit32 {
                frame.apply_color_depth(dithering);
            }
            frame.dirty = true;
            self.status_msg = format!(
                "Decupat frame {}x{} ({}).",
                frame.width,
                frame.height,
                frame.color_depth.short_label()
            );
        }
    }

    pub(crate) fn edit_paste(&mut self) {
        let Some((clip_w, clip_h, clip_pixels)) = self.clipboard.clone() else {
            self.status_msg = "Clipboard-ul este gol.".to_string();
            return;
        };

        let (dest_x, dest_y) = match self.selection {
            Some((x0, y0, _, _)) => (x0, y0),
            None => (0, 0),
        };

        let dithering = self.use_dithering;
        let frame = &mut self.frames[self.active_frame];
        frame.push_undo();

        for sy in 0..clip_h {
            let ty = dest_y + sy;
            if ty >= frame.height {
                break;
            }
            for sx in 0..clip_w {
                let tx = dest_x + sx;
                if tx >= frame.width {
                    break;
                }
                let px = clip_pixels[(sy * clip_w + sx) as usize];
                frame.set_pixel(tx, ty, px);
            }
        }

        if frame.color_depth != ColorDepth::Bit32 {
            frame.apply_color_depth(dithering);
        }
        frame.dirty = true;

        let end_x = (dest_x + clip_w - 1).min(frame.width - 1);
        let end_y = (dest_y + clip_h - 1).min(frame.height - 1);
        self.selection = Some((dest_x, dest_y, end_x, end_y));

        self.status_msg = format!("Lipit {}x{} la ({}, {}).", clip_w, clip_h, dest_x, dest_y);
    }

    pub(crate) fn select_all(&mut self) {
        let frame = &self.frames[self.active_frame];
        self.selection = Some((0, 0, frame.width - 1, frame.height - 1));
        self.status_msg = format!("Selectat tot frame-ul ({}x{}).", frame.width, frame.height);
    }

    pub(crate) fn deselect(&mut self) {
        self.selection = None;
        self.select_drag_start = None;
        self.status_msg = "Selectie anulata.".to_string();
    }

    pub(crate) fn clear_selection_pixels(&mut self) {
        let Some((x0, y0, x1, y1)) = self.selection else {
            return;
        };
        let dithering = self.use_dithering;
        let frame = &mut self.frames[self.active_frame];
        frame.push_undo();
        for y in y0..=y1 {
            for x in x0..=x1 {
                frame.set_pixel(x, y, [0, 0, 0, 0]);
            }
        }
        if frame.color_depth != ColorDepth::Bit32 {
            frame.apply_color_depth(dithering);
        }
        frame.dirty = true;
        self.status_msg = format!(
            "Sters zona selectata ({}x{}).",
            x1 - x0 + 1,
            y1 - y0 + 1
        );
    }

    pub(crate) fn get_checker_texture(&mut self, ctx: &egui::Context) -> egui::TextureId {
        let tex = self.checker_texture.get_or_insert_with(|| {
            let c1 = [55, 55, 55, 255];
            let c2 = [38, 38, 38, 255];
            let pixels = [c1, c2, c2, c1].concat();
            let img = egui::ColorImage::from_rgba_unmultiplied([2, 2], &pixels);
            let opts = egui::TextureOptions {
                wrap_mode: egui::TextureWrapMode::Repeat,
                ..egui::TextureOptions::NEAREST
            };
            ctx.load_texture("checkerboard_bg", img, opts)
        });
        tex.id()
    }

    pub(crate) fn sort_frames(&mut self) {
        self.frames
            .sort_by(|a, b| b.width.cmp(&a.width).then_with(|| b.color_depth.cmp(&a.color_depth)));
    }

    pub(crate) fn import_png(&mut self, path: &Path) {
        match image::open(path) {
            Ok(img) => {
                self.frames = STANDARD_SIZES
                    .iter()
                    .rev()
                    .map(|&size| {
                        let resized = img
                            .resize_exact(size, size, FilterType::Lanczos3)
                            .to_rgba8();
                        IconFrame::from_rgba(size, size, ColorDepth::Bit32, resized.into_raw())
                    })
                    .collect();
                self.active_frame = 0;
                self.auto_fit_zoom();
                self.status_msg = format!(
                    "Importat PNG (32-bit): {:?}",
                    path.file_name().unwrap_or_default()
                );
            }
            Err(e) => self.status_msg = format!("Eroare import PNG: {e}"),
        }
    }

    pub(crate) fn open_ico(&mut self, path: &Path) {
        let Ok(file) = File::open(path) else {
            self.status_msg = "Nu s-a putut deschide fisierul .ico".to_string();
            return;
        };
        match IconDir::read(file) {
            Ok(dir) => {
                let mut loaded = Vec::new();
                for entry in dir.entries() {
                    let bpp = entry.bits_per_pixel();
                    let depth = if entry.is_png() && bpp == 0 {
                        ColorDepth::Bit32
                    } else {
                        ColorDepth::from_bits(bpp)
                    };
                    if let Ok(img) = entry.decode() {
                        loaded.push(IconFrame::from_rgba(
                            img.width(),
                            img.height(),
                            depth,
                            img.rgba_data().to_vec(),
                        ));
                    }
                }
                if !loaded.is_empty() {
                    self.frames = loaded;
                    self.sort_frames();
                    self.active_frame = 0;
                    self.current_file = Some(path.to_path_buf());
                    self.auto_fit_zoom();
                    self.status_msg = format!(
                        "Deschis .ico ({} variante dimensiune/culoare)",
                        self.frames.len()
                    );
                }
            }
            Err(e) => self.status_msg = format!("Eroare parsare .ico: {e}"),
        }
    }

    pub(crate) fn save_ico(&mut self, path: &Path) {
        if self.frames.is_empty() {
            return;
        }

        let mut payloads = Vec::with_capacity(self.frames.len());
        for frame in &self.frames {
            match frame.encode_ico_payload(self.use_dithering) {
                Ok(data) => payloads.push(data),
                Err(e) => {
                    self.status_msg = format!(
                        "Eroare codare {}x{} ({}): {e}",
                        frame.width,
                        frame.height,
                        frame.color_depth.short_label()
                    );
                    return;
                }
            }
        }

        let count = self.frames.len() as u16;
        let header_size = 6usize + self.frames.len() * 16;
        let mut current_offset = header_size;
        let mut out = Vec::new();

        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());

        for (frame, payload) in self.frames.iter().zip(payloads.iter()) {
            let w_byte = if frame.width >= 256 {
                0u8
            } else {
                frame.width as u8
            };
            let h_byte = if frame.height >= 256 {
                0u8
            } else {
                frame.height as u8
            };
            let color_count = match frame.color_depth {
                ColorDepth::Bit1 => 2u8,
                ColorDepth::Bit4 => 16u8,
                _ => 0u8,
            };

            out.push(w_byte);
            out.push(h_byte);
            out.push(color_count);
            out.push(0u8);
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&frame.color_depth.bits().to_le_bytes());
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            out.extend_from_slice(&(current_offset as u32).to_le_bytes());

            current_offset += payload.len();
        }

        for payload in payloads {
            out.extend_from_slice(&payload);
        }

        match File::create(path).and_then(|mut f| f.write_all(&out)) {
            Ok(_) => {
                self.current_file = Some(path.to_path_buf());
                self.status_msg = format!(
                    "Salvat .ICO ({} variante): {:?}",
                    self.frames.len(),
                    path.file_name().unwrap_or_default()
                )
            }
            Err(e) => self.status_msg = format!("Eroare scriere fisier: {e}"),
        }
    }

    pub(crate) fn add_or_replace_variant(&mut self, size: u32, depth: ColorDepth) {
        let source_idx = self
            .frames
            .iter()
            .enumerate()
            .max_by_key(|(_, f)| {
                (
                    if f.color_depth == ColorDepth::Bit32 {
                        1
                    } else {
                        0
                    },
                    f.width,
                )
            })
            .map(|(i, _)| i)
            .unwrap_or(self.active_frame);

        let src = &self.frames[source_idx];
        let Some(base_img) =
            image::RgbaImage::from_raw(src.width, src.height, src.to_flat_rgba())
        else {
            return;
        };

        let resized = if src.width == size && src.height == size {
            base_img
        } else {
            image::DynamicImage::ImageRgba8(base_img)
                .resize_exact(size, size, FilterType::Lanczos3)
                .to_rgba8()
        };

        let mut new_frame = IconFrame::from_rgba(size, size, depth, resized.into_raw());
        new_frame.apply_color_depth(self.use_dithering);

        if let Some(pos) = self
            .frames
            .iter()
            .position(|f| f.width == size && f.color_depth == depth)
        {
            self.frames[pos] = new_frame;
        } else {
            self.frames.push(new_frame);
        }

        self.sort_frames();
        if let Some(pos) = self
            .frames
            .iter()
            .position(|f| f.width == size && f.color_depth == depth)
        {
            self.active_frame = pos;
            self.auto_fit_zoom();
        }
    }

    pub(crate) fn generate_full_windows_set(&mut self) {
        let targets = [
            (256, ColorDepth::Bit32),
            (48, ColorDepth::Bit32),
            (48, ColorDepth::Bit8),
            (48, ColorDepth::Bit4),
            (32, ColorDepth::Bit32),
            (32, ColorDepth::Bit8),
            (32, ColorDepth::Bit4),
            (16, ColorDepth::Bit32),
            (16, ColorDepth::Bit8),
            (16, ColorDepth::Bit4),
        ];

        for (size, depth) in targets {
            self.add_or_replace_variant(size, depth);
        }

        self.active_frame = 0;
        self.auto_fit_zoom();
        self.status_msg = format!(
            "Generat set Windows complet ({} variante: 32-bit, 8-bit, 4-bit).",
            self.frames.len()
        );
    }

    pub(crate) fn regenerate_from_current(&mut self) {
        let current = &self.frames[self.active_frame];
        let Some(base_img) =
            image::RgbaImage::from_raw(current.width, current.height, current.to_flat_rgba())
        else {
            return;
        };
        let dyn_img = image::DynamicImage::ImageRgba8(base_img);
        let dithering = self.use_dithering;

        for (idx, frame) in self.frames.iter_mut().enumerate() {
            if idx == self.active_frame {
                continue;
            }
            frame.push_undo();
            let resized = dyn_img
                .resize_exact(frame.width, frame.height, FilterType::Lanczos3)
                .to_rgba8();
            let depth = frame.color_depth;
            let mut updated =
                IconFrame::from_rgba(frame.width, frame.height, depth, resized.into_raw());
            updated.apply_color_depth(dithering);
            *frame = updated;
        }
        self.status_msg =
            "Toate rezolutiile si adancimile au fost regenerate din frame-ul curent.".to_string();
    }

    pub(crate) fn auto_fit_zoom(&mut self) {
        let w = self.frames[self.active_frame].width as f32;
        self.zoom = (512.0 / w).clamp(2.0, 28.0);
    }
}