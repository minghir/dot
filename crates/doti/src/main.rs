#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use ico::IconDir;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::ImageEncoder;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

const STANDARD_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum ColorDepth {
    Bit1,
    Bit4,
    Bit8,
    Bit24,
    Bit32,
}

impl ColorDepth {
    const ALL: [ColorDepth; 5] = [
        ColorDepth::Bit32,
        ColorDepth::Bit24,
        ColorDepth::Bit8,
        ColorDepth::Bit4,
        ColorDepth::Bit1,
    ];

    fn bits(self) -> u16 {
        match self {
            ColorDepth::Bit1 => 1,
            ColorDepth::Bit4 => 4,
            ColorDepth::Bit8 => 8,
            ColorDepth::Bit24 => 24,
            ColorDepth::Bit32 => 32,
        }
    }

    fn short_label(self) -> &'static str {
        match self {
            ColorDepth::Bit1 => "1-bit",
            ColorDepth::Bit4 => "4-bit",
            ColorDepth::Bit8 => "8-bit",
            ColorDepth::Bit24 => "24-bit",
            ColorDepth::Bit32 => "32-bit",
        }
    }

    fn full_label(self) -> &'static str {
        match self {
            ColorDepth::Bit1 => "1-bit (2 culori)",
            ColorDepth::Bit4 => "4-bit (16 culori)",
            ColorDepth::Bit8 => "8-bit (256 culori)",
            ColorDepth::Bit24 => "24-bit (RGB + masca)",
            ColorDepth::Bit32 => "32-bit (RGBA Alpha)",
        }
    }

    fn from_bits(bits: u16) -> Self {
        match bits {
            1 => ColorDepth::Bit1,
            4 => ColorDepth::Bit4,
            8 => ColorDepth::Bit8,
            24 => ColorDepth::Bit24,
            _ => ColorDepth::Bit32,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    SelectRect,
}


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
    // Găleată înclinată
    let p1 = egui::pos2(rect.min.x + 8.0_f32, rect.min.y + 3.0_f32);
    let p2 = egui::pos2(rect.max.x - 3.0_f32, rect.min.y + 8.0_f32);
    let p3 = egui::pos2(rect.min.x + 8.0_f32, rect.max.y - 3.0_f32);
    let p4 = egui::pos2(rect.min.x + 3.0_f32, rect.min.y + 8.0_f32);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    painter.line_segment([p3, p4], stroke);
    painter.line_segment([p4, p1], stroke);

    // Picătură în dreapta
    painter.circle_filled(
        egui::pos2(rect.max.x - 2.5_f32, rect.max.y - 4.0_f32),
        1.8_f32,
        color,
    );
}

fn draw_eyedropper_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.5_f32, color);
    // Vârf pipetă jos-stânga -> corp sus-dreapta
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


struct IconFrame {
    width: u32,
    height: u32,
    color_depth: ColorDepth,
    pixels: Vec<[u8; 4]>, // RGBA8
    undo_stack: Vec<Vec<[u8; 4]>>,
    redo_stack: Vec<Vec<[u8; 4]>>,
    texture: Option<egui::TextureHandle>,
    dirty: bool,
}




impl IconFrame {
    fn remove_corner_background(&mut self, tolerance: f32) {
        let w = self.width as usize;
        let h = self.height as usize;
        if w == 0 || h == 0 {
            return;
        }
        let bg = self.get_pixel(0, 0);
        let mut visited = vec![false; w * h];
        let mut stack = vec![
            (0u32, 0u32),
            (self.width - 1, 0),
            (0, self.height - 1),
            (self.width - 1, self.height - 1),
        ];
        let feather = 30.0_f32;

        while let Some((x, y)) = stack.pop() {
            let idx = y as usize * w + x as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            let p = self.pixels[idx];
            if p[3] == 0 {
                continue;
            }

            let dr = p[0] as f32 - bg[0] as f32;
            let dg = p[1] as f32 - bg[1] as f32;
            let db = p[2] as f32 - bg[2] as f32;
            let dist = (dr * dr + dg * dg + db * db).sqrt();

            if dist <= tolerance {
                self.pixels[idx] = [0, 0, 0, 0];
                if x > 0 { stack.push((x - 1, y)); }
                if x + 1 < self.width { stack.push((x + 1, y)); }
                if y > 0 { stack.push((x, y - 1)); }
                if y + 1 < self.height { stack.push((x, y + 1)); }
            } else if dist < tolerance + feather {
                // Elimină haloul alb de pe marginile rotunjite
                let alpha = ((dist - tolerance) / feather).clamp(0.0, 1.0);
                let unblend = |c: u8, b: u8| -> u8 {
                    (((c as f32 - (1.0 - alpha) * b as f32) / alpha).clamp(0.0, 255.0)) as u8
                };
                self.pixels[idx] = [
                    unblend(p[0], bg[0]),
                    unblend(p[1], bg[1]),
                    unblend(p[2], bg[2]),
                    (alpha * 255.0) as u8,
                ];
            }
        }
        self.dirty = true;
    }


    fn new(width: u32, height: u32, color_depth: ColorDepth) -> Self {
        let count = (width * height) as usize;
        Self {
            width,
            height,
            color_depth,
            pixels: vec![[0, 0, 0, 0]; count],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            texture: None,
            dirty: true,
        }
    }

    fn from_rgba(width: u32, height: u32, color_depth: ColorDepth, raw: Vec<u8>) -> Self {
        let pixels: Vec<[u8; 4]> = raw
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();
        Self {
            width,
            height,
            color_depth,
            pixels,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            texture: None,
            dirty: true,
        }
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= 30 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.pixels.clone());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.pixels.clone());
            self.pixels = prev;
            self.dirty = true;
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.pixels.clone());
            self.pixels = next;
            self.dirty = true;
        }
    }

    fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixels[(y * self.width + x) as usize]
    }

    fn set_pixel(&mut self, x: u32, y: u32, mut color: [u8; 4]) {
        // Pentru adâncimi < 32-bit, transparența este binară (0 sau 255)
        if self.color_depth != ColorDepth::Bit32 {
            color[3] = if color[3] < 128 { 0 } else { 255 };
        }
        let idx = (y * self.width + x) as usize;
        if self.pixels[idx] != color {
            self.pixels[idx] = color;
            self.dirty = true;
        }
    }

    fn flood_fill(&mut self, start_x: u32, start_y: u32, mut new_color: [u8; 4]) {
        if self.color_depth != ColorDepth::Bit32 {
            new_color[3] = if new_color[3] < 128 { 0 } else { 255 };
        }
        let target = self.get_pixel(start_x, start_y);
        if target == new_color {
            return;
        }
        let mut stack = vec![(start_x, start_y)];
        while let Some((x, y)) = stack.pop() {
            if self.get_pixel(x, y) != target {
                continue;
            }
            self.set_pixel(x, y, new_color);
            if x > 0 {
                stack.push((x - 1, y));
            }
            if x + 1 < self.width {
                stack.push((x + 1, y));
            }
            if y > 0 {
                stack.push((x, y - 1));
            }
            if y + 1 < self.height {
                stack.push((x, y + 1));
            }
        }
    }

    fn to_flat_rgba(&self) -> Vec<u8> {
        let mut flat = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            flat.extend_from_slice(p);
        }
        flat
    }

    fn update_texture(&mut self, ctx: &egui::Context, index: usize) {
        if !self.dirty && self.texture.is_some() {
            return;
        }
        let flat = self.to_flat_rgba();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [self.width as usize, self.height as usize],
            &flat,
        );
        let name = format!(
            "frame_{}_{}x{}_{}",
            index,
            self.width,
            self.height,
            self.color_depth.bits()
        );
        self.texture = Some(ctx.load_texture(name, color_image, egui::TextureOptions::NEAREST));
        self.dirty = false;
    }

    /// Cuantizează pixelii frame-ului conform `self.color_depth` (pentru preview WYSIWYG și editare)
    fn apply_color_depth(&mut self, dithering: bool) {
        match self.color_depth {
            ColorDepth::Bit32 => {}
            ColorDepth::Bit24 => {
                for p in &mut self.pixels {
                    if p[3] < 128 {
                        *p = [0, 0, 0, 0];
                    } else {
                        p[3] = 255;
                    }
                }
                self.dirty = true;
            }
            ColorDepth::Bit8 | ColorDepth::Bit4 | ColorDepth::Bit1 => {
                let max_colors = 1usize << self.color_depth.bits();
                let palette = build_palette(&self.pixels, max_colors);
                let (quantized, _) =
                    quantize_pixels(&self.pixels, self.width, self.height, &palette, dithering);
                self.pixels = quantized;
                self.dirty = true;
            }
        }
    }

    /// Encodează payload-ul binar (PNG pentru 256x256@32-bit, sau DIB BMP + AND mask pentru restul)
    fn encode_ico_payload(&self, dithering: bool) -> Result<Vec<u8>, String> {
        // Windows Vista/7/10/11 folosește PNG în interiorul .ico pentru 256x256 32-bit
        if self.width == 256 && self.height == 256 && self.color_depth == ColorDepth::Bit32 {
            let mut png_bytes = Vec::new();
            let encoder = PngEncoder::new(&mut png_bytes);
            encoder
                .write_image(
                    &self.to_flat_rgba(),
                    self.width,
                    self.height,
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(|e| format!("Eroare PNG 256x256: {e}"))?;
            return Ok(png_bytes);
        }

        let w = self.width as usize;
        let h = self.height as usize;
        let bits = self.color_depth.bits();

        // Pregătim paleta și indicii pentru modurile indexate (1, 4, 8-bit)
        let (palette, indices) = match self.color_depth {
            ColorDepth::Bit1 | ColorDepth::Bit4 | ColorDepth::Bit8 => {
                let max_colors = 1usize << bits;
                let pal = build_palette(&self.pixels, max_colors);
                let (_, idxs) =
                    quantize_pixels(&self.pixels, self.width, self.height, &pal, dithering);
                (pal, idxs)
            }
            _ => (Vec::new(), Vec::new()),
        };

        // Calculăm dimensiunile măștilor XOR și AND (aliniate la 4 bytes per rând)
        let xor_stride = match bits {
            32 => w * 4,
            24 => (w * 3 + 3) & !3,
            8 => (w + 3) & !3,
            4 => ((w + 1) / 2 + 3) & !3,
            1 => ((w + 7) / 8 + 3) & !3,
            _ => unreachable!(),
        };
        let and_stride = ((w + 7) / 8 + 3) & !3;

        let xor_size = xor_stride * h;
        let and_size = and_stride * h;
        let palette_bytes = palette.len() * 4;

        let mut buf = Vec::with_capacity(40 + palette_bytes + xor_size + and_size);

        // 1. BITMAPINFOHEADER (40 bytes)
        buf.extend_from_slice(&40u32.to_le_bytes()); // biSize
        buf.extend_from_slice(&(self.width as i32).to_le_bytes()); // biWidth
        buf.extend_from_slice(&((self.height * 2) as i32).to_le_bytes()); // biHeight (XOR + AND)
        buf.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
        buf.extend_from_slice(&bits.to_le_bytes()); // biBitCount
        buf.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
        buf.extend_from_slice(&((xor_size + and_size) as u32).to_le_bytes()); // biSizeImage
        buf.extend_from_slice(&0i32.to_le_bytes()); // biXPelsPerMeter
        buf.extend_from_slice(&0i32.to_le_bytes()); // biYPelsPerMeter
        let clr_used = if palette.is_empty() {
            0u32
        } else {
            palette.len() as u32
        };
        buf.extend_from_slice(&clr_used.to_le_bytes()); // biClrUsed
        buf.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant

        // 2. Color Table (RGBQUAD: B, G, R, 0) pentru 1, 4, 8-bit
        for [r, g, b] in &palette {
            buf.extend_from_slice(&[*b, *g, *r, 0]);
        }

        // 3. XOR Mask (stocat bottom-up, de la y = h - 1 până la 0)
        for y in (0..h).rev() {
            let row_start = buf.len();
            match bits {
                32 => {
                    for x in 0..w {
                        let [r, g, b, a] = self.pixels[y * w + x];
                        buf.extend_from_slice(&[b, g, r, a]);
                    }
                }
                24 => {
                    for x in 0..w {
                        let [r, g, b, a] = self.pixels[y * w + x];
                        if a < 128 {
                            buf.extend_from_slice(&[0, 0, 0]);
                        } else {
                            buf.extend_from_slice(&[b, g, r]);
                        }
                    }
                }
                8 => {
                    for x in 0..w {
                        buf.push(indices[y * w + x]);
                    }
                }
                4 => {
                    for x in (0..w).step_by(2) {
                        let hi = indices[y * w + x] & 0x0F;
                        let lo = if x + 1 < w {
                            indices[y * w + x + 1] & 0x0F
                        } else {
                            0
                        };
                        buf.push((hi << 4) | lo);
                    }
                }
                1 => {
                    let mut byte = 0u8;
                    for x in 0..w {
                        let bit = indices[y * w + x] & 1;
                        byte |= bit << (7 - (x % 8));
                        if x % 8 == 7 || x + 1 == w {
                            buf.push(byte);
                            byte = 0;
                        }
                    }
                }
                _ => unreachable!(),
            }
            while buf.len() - row_start < xor_stride {
                buf.push(0);
            }
        }

        // 4. AND Mask (1-bit transparență, bottom-up: 1 = transparent, 0 = opac)
        for y in (0..h).rev() {
            let row_start = buf.len();
            let mut byte = 0u8;
            for x in 0..w {
                let a = self.pixels[y * w + x][3];
                if a < 128 {
                    byte |= 1 << (7 - (x % 8));
                }
                if x % 8 == 7 || x + 1 == w {
                    buf.push(byte);
                    byte = 0;
                }
            }
            while buf.len() - row_start < and_stride {
                buf.push(0);
            }
        }

        Ok(buf)
    }
}

/// Construiește o paletă optimă folosind algoritmul Median Cut.
/// Indexul 0 este rezervat pentru `[0, 0, 0]` (folosit de pixelii transparenți în masca XOR Windows).
fn build_palette(pixels: &[[u8; 4]], max_colors: usize) -> Vec<[u8; 3]> {
    if max_colors <= 2 {
        return vec![[0, 0, 0], [255, 255, 255]];
    }

    let opaque: Vec<[u8; 3]> = pixels
        .iter()
        .filter(|p| p[3] >= 128)
        .map(|p| [p[0], p[1], p[2]])
        .collect();

    if opaque.is_empty() {
        return vec![[0, 0, 0]; max_colors];
    }

    let target_opaque = max_colors - 1;
    let mut buckets: Vec<Vec<[u8; 3]>> = vec![opaque];

    while buckets.len() < target_opaque {
        let mut best_idx = None;
        let mut best_range = 0u8;
        let mut best_channel = 0usize;

        for (i, b) in buckets.iter().enumerate() {
            if b.len() < 2 {
                continue;
            }
            let (mut min_c, mut max_c) = ([255u8; 3], [0u8; 3]);
            for px in b {
                for ch in 0..3 {
                    min_c[ch] = min_c[ch].min(px[ch]);
                    max_c[ch] = max_c[ch].max(px[ch]);
                }
            }
            for ch in 0..3 {
                let range = max_c[ch] - min_c[ch];
                if range > best_range {
                    best_range = range;
                    best_idx = Some(i);
                    best_channel = ch;
                }
            }
        }

        let Some(idx) = best_idx else {
            break;
        };

        buckets[idx].sort_by_key(|px| px[best_channel]);
        let mid = buckets[idx].len() / 2;
        let right = buckets[idx].split_off(mid);
        buckets.push(right);
    }

    let mut palette = Vec::with_capacity(max_colors);
    palette.push([0, 0, 0]); // Index 0 = transparent XOR

    for b in buckets {
        if b.is_empty() {
            continue;
        }
        let (mut sr, mut sg, mut sb) = (0u64, 0u64, 0u64);
        for [r, g, b_val] in &b {
            sr += *r as u64;
            sg += *g as u64;
            sb += *b_val as u64;
        }
        let n = b.len() as u64;
        palette.push([(sr / n) as u8, (sg / n) as u8, (sb / n) as u8]);
    }

    while palette.len() < max_colors {
        palette.push([0, 0, 0]);
    }
    palette
}

fn nearest_palette_index(rgb: [u8; 3], palette: &[[u8; 3]], start_idx: usize) -> usize {
    let mut best_idx = start_idx;
    let mut best_dist = i32::MAX;
    for (i, &[pr, pg, pb]) in palette.iter().enumerate().skip(start_idx) {
        let dr = rgb[0] as i32 - pr as i32;
        let dg = rgb[1] as i32 - pg as i32;
        let db = rgb[2] as i32 - pb as i32;
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i;
        }
    }
    best_idx
}

fn quantize_pixels(
    pixels: &[[u8; 4]],
    width: u32,
    height: u32,
    palette: &[[u8; 3]],
    dithering: bool,
) -> (Vec<[u8; 4]>, Vec<u8>) {
    let w = width as usize;
    let h = height as usize;
    let mut out_rgba = vec![[0u8; 4]; w * h];
    let mut out_idx = vec![0u8; w * h];

    // Pentru 1-bit căutăm de la index 0 (negru/alb), pentru 4/8-bit de la index 1 (0 e transparent)
    let start_idx = if palette.len() <= 2 { 0 } else { 1 };

    if !dithering {
        for (i, &[r, g, b, a]) in pixels.iter().enumerate() {
            if a < 128 {
                out_rgba[i] = [0, 0, 0, 0];
                out_idx[i] = 0;
            } else {
                let idx = nearest_palette_index([r, g, b], palette, start_idx);
                let [pr, pg, pb] = palette[idx];
                out_rgba[i] = [pr, pg, pb, 255];
                out_idx[i] = idx as u8;
            }
        }
        return (out_rgba, out_idx);
    }

    // Floyd-Steinberg Error Diffusion
    let mut buf: Vec<[f32; 3]> = pixels
        .iter()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if pixels[i][3] < 128 {
                out_rgba[i] = [0, 0, 0, 0];
                out_idx[i] = 0;
                continue;
            }

            let old = [
                buf[i][0].clamp(0.0, 255.0) as u8,
                buf[i][1].clamp(0.0, 255.0) as u8,
                buf[i][2].clamp(0.0, 255.0) as u8,
            ];
            let idx = nearest_palette_index(old, palette, start_idx);
            let chosen = palette[idx];
            out_rgba[i] = [chosen[0], chosen[1], chosen[2], 255];
            out_idx[i] = idx as u8;

            let err = [
                buf[i][0] - chosen[0] as f32,
                buf[i][1] - chosen[1] as f32,
                buf[i][2] - chosen[2] as f32,
            ];

            let distribute = |buf: &mut Vec<[f32; 3]>, nx: isize, ny: isize, factor: f32| {
                if nx >= 0 && nx < w as isize && ny >= 0 && ny < h as isize {
                    let ni = ny as usize * w + nx as usize;
                    if pixels[ni][3] >= 128 {
                        buf[ni][0] += err[0] * factor;
                        buf[ni][1] += err[1] * factor;
                        buf[ni][2] += err[2] * factor;
                    }
                }
            };

            let (ix, iy) = (x as isize, y as isize);
            distribute(&mut buf, ix + 1, iy, 7.0 / 16.0);
            distribute(&mut buf, ix - 1, iy + 1, 3.0 / 16.0);
            distribute(&mut buf, ix, iy + 1, 5.0 / 16.0);
            distribute(&mut buf, ix + 1, iy + 1, 1.0 / 16.0);
        }
    }

    (out_rgba, out_idx)
}

struct IconEditorApp {
    frames: Vec<IconFrame>,
    active_frame: usize,
    tool: Tool,
    primary_color: [u8; 4],
    zoom: f32,
    show_grid: bool,
    use_dithering: bool,
    new_frame_size: u32,
    new_frame_depth: ColorDepth,
    last_draw_pos: Option<(i32, i32)>,
    status_msg: String,
    current_file: Option<PathBuf>,
    clipboard: Option<(u32, u32, Vec<[u8; 4]>)>, // (width, height, pixels)
    checker_texture: Option<egui::TextureHandle>,
    selection: Option<(u32, u32, u32, u32)>, // (x0, y0, x1, y1) inclusiv, în coordonate pixel
    select_drag_start: Option<(i32, i32)>,
    hovered_coord: Option<(u32, u32)>,
    cmd_input: String,
    cmd_history: Vec<String>,
    cmd_vars: std::collections::HashMap<String, dot_lang::Value>,
    cmd_funcs: std::collections::HashMap<String, dot_lang::UserFn>,
    cmd_structs: std::collections::HashMap<String, dot_lang::StructDef>,
    dot_script_path: String,
}

impl Default for IconEditorApp {
    fn default() -> Self {
        let frames = STANDARD_SIZES
            .iter()
            .map(|&s| IconFrame::new(s, s, ColorDepth::Bit32))
            .collect();
        Self {
            frames,
            active_frame: 2, // 32x32 @ 32-bit implicit
            tool: Tool::Pencil,
            primary_color: [255, 59, 48, 255], // Roșu RED (#FF3B30)
            zoom: 16.0,
            show_grid: true,
            use_dithering: true,
            new_frame_size: 32,
            new_frame_depth: ColorDepth::Bit8,
            last_draw_pos: None,
            status_msg: "Gata. Importa un PNG sau deseneaza direct pe grid.".to_string(),
            current_file: None,
            clipboard: None,
            checker_texture: None,
            selection: None,
            select_drag_start: None,
            hovered_coord: None,
            cmd_input: String::new(),
            cmd_history: Vec::new(),
            cmd_vars: std::collections::HashMap::new(), 
            cmd_funcs: std::collections::HashMap::new(),
            cmd_structs: std::collections::HashMap::new(),
            dot_script_path: "fractal_tree.dot".to_string(),
        }
    }
}

impl IconEditorApp {

    fn run_dot_command(&mut self, raw: &str) {
        let line = raw.trim();
        if line.is_empty() {
            return;
        }

        self.cmd_history.push(format!("> {line}"));

        let frame = &mut self.frames[self.active_frame];
        frame.push_undo();

        let saved_vars = std::mem::take(&mut self.cmd_vars);
        let saved_funcs = std::mem::take(&mut self.cmd_funcs);
        let saved_structs = std::mem::take(&mut self.cmd_structs);

        let (result, returned_vars, returned_funcs, returned_structs) = {
            let Ok(canvas) =
                dot_lang::CanvasBuffer::new(frame.width, frame.height, &mut frame.pixels)
            else {
                self.cmd_vars = saved_vars;
                self.cmd_funcs = saved_funcs;
                self.cmd_structs = saved_structs;
                self.cmd_history
                    .push("Eroare: buffer de pixeli invalid.".to_string());
                return;
            };

            let mut ctx = dot_lang::EvalContext {
                canvas,
                output: Vec::new(),
                vars: saved_vars,
                funcs: saved_funcs,
                structs: saved_structs,
                call_depth: 0,
            };

            let eval_res = dot_lang::eval_script(line, &mut ctx);
            (eval_res.map(|()| ctx.output), ctx.vars, ctx.funcs, ctx.structs)
        };

        self.cmd_vars = returned_vars;
        self.cmd_funcs = returned_funcs;
        self.cmd_structs = returned_structs;
        frame.dirty = true;

        match result {
            Ok(lines) => {
                for l in lines {
                    self.cmd_history.push(l);
                }
                self.status_msg = "Script executat.".to_string();
            }
            Err(err) => {
                let msg = format!("Eroare dot: {err}");
                self.cmd_history.push(msg.clone());
                self.status_msg = msg;
            }
        }
    }

    fn load_dot_script_from_path(&mut self, auto_run: bool) {
        let trimmed = self.dot_script_path.trim().trim_matches('"');
        if trimmed.is_empty() {
            self.cmd_history
                .push("Eroare dot: introdu calea către un fișier .dot.".to_string());
            return;
        }

        match std::fs::read_to_string(trimmed) {
            Ok(contents) => {
                self.cmd_input = contents.clone();
                self.cmd_history
                    .push(format!("[încărcat: {trimmed}]"));
                if auto_run {
                    self.run_dot_command(&contents);
                    self.cmd_input.clear();
                } else {
                    self.status_msg = format!("Script încărcat în consolă: {trimmed}");
                }
            }
            Err(err) => {
                let msg = format!("Eroare la citirea '{trimmed}': {err}");
                self.cmd_history.push(msg.clone());
                self.status_msg = msg;
            }
        }
    }
    
    fn from_startup_path(path: Option<PathBuf>) -> IconEditorApp {
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
   

    fn save_current_ico(&mut self) {
        if let Some(path) = self.current_file.clone() {
            self.save_ico(&path);
        } else {
            self.save_ico_as();
        }
    }

    fn save_ico_as(&mut self) {
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

    fn close_file(&mut self) {
        *self = Self::default();
        self.status_msg = "Fisier inchis.".to_string();
    }

    fn export_active_png(&mut self) {
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

    fn edit_undo(&mut self) {
        self.frames[self.active_frame].undo();
        self.status_msg = "Undo aplicat.".to_string();
    }

    fn edit_redo(&mut self) {
        self.frames[self.active_frame].redo();
        self.status_msg = "Redo aplicat.".to_string();
    }

    fn edit_copy(&mut self) {
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

    fn edit_cut(&mut self) {
        // Mai întâi copiem zona selectată (sau tot frame-ul dacă nu există selecție)
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

    fn edit_paste(&mut self) {
        let Some((clip_w, clip_h, clip_pixels)) = self.clipboard.clone() else {
            self.status_msg = "Clipboard-ul este gol.".to_string();
            return;
        };

        // Lipim în colțul stânga-sus al selecției curente (sau la (0, 0) dacă nu e nimic selectat)
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

        // Actualizăm chenarul de selecție pe zona proaspăt lipită
        let end_x = (dest_x + clip_w - 1).min(frame.width - 1);
        let end_y = (dest_y + clip_h - 1).min(frame.height - 1);
        self.selection = Some((dest_x, dest_y, end_x, end_y));

        self.status_msg = format!("Lipit {}x{} la ({}, {}).", clip_w, clip_h, dest_x, dest_y);
    }

    fn select_all(&mut self) {
        let frame = &self.frames[self.active_frame];
        self.selection = Some((0, 0, frame.width - 1, frame.height - 1));
        self.status_msg = format!("Selectat tot frame-ul ({}x{}).", frame.width, frame.height);
    }

    fn deselect(&mut self) {
        self.selection = None;
        self.select_drag_start = None;
        self.status_msg = "Selectie anulata.".to_string();
    }

    fn clear_selection_pixels(&mut self) {
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

    fn get_checker_texture(&mut self, ctx: &egui::Context) -> egui::TextureId {
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

    fn sort_frames(&mut self) {
        self.frames
            .sort_by(|a, b| b.width.cmp(&a.width).then_with(|| b.color_depth.cmp(&a.color_depth)));
    }

    fn import_png(&mut self, path: &Path) {
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
                self.active_frame = 0; // Selectează 256x256 @ 32-bit
                self.auto_fit_zoom();
                self.status_msg = format!(
                    "Importat PNG (32-bit): {:?}",
                    path.file_name().unwrap_or_default()
                );
            }
            Err(e) => self.status_msg = format!("Eroare import PNG: {e}"),
        }
    }

    fn open_ico(&mut self, path: &Path) {
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

    fn save_ico(&mut self, path: &Path) {
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

        // ICONDIR (6 bytes)
        out.extend_from_slice(&0u16.to_le_bytes()); // Reserved
        out.extend_from_slice(&1u16.to_le_bytes()); // Type = 1 (ICO)
        out.extend_from_slice(&count.to_le_bytes()); // Count

        // ICONDIRENTRY (16 bytes per imagine)
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
                _ => 0u8, // 0 pentru 8-bit, 24-bit și 32-bit conform specificației MS ICO
            };

            out.push(w_byte);
            out.push(h_byte);
            out.push(color_count);
            out.push(0u8); // Reserved
            out.extend_from_slice(&1u16.to_le_bytes()); // Planes
            out.extend_from_slice(&frame.color_depth.bits().to_le_bytes()); // BitCount
            out.extend_from_slice(&(payload.len() as u32).to_le_bytes()); // BytesInRes
            out.extend_from_slice(&(current_offset as u32).to_le_bytes()); // ImageOffset

            current_offset += payload.len();
        }

        // Datele efective ale imaginilor
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

    fn add_or_replace_variant(&mut self, size: u32, depth: ColorDepth) {
        // Luăm sursa cea mai bună disponibilă (preferabil 32-bit de rezoluție cât mai mare)
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

    /// Generează automat matricea clasică Windows (256@32bit + 48/32/16 @ 32bit, 8bit, 4bit)
    fn generate_full_windows_set(&mut self) {
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

    fn regenerate_from_current(&mut self) {
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

    fn auto_fit_zoom(&mut self) {
        let w = self.frames[self.active_frame].width as f32;
        self.zoom = (512.0 / w).clamp(2.0, 28.0);
    }
}

fn plot_line(x0: i32, y0: i32, x1: i32, y1: i32, mut plot: impl FnMut(i32, i32)) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        plot(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

impl eframe::App for IconEditorApp {
    

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let title = match &self.current_file {
            Some(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("icon.ico");
                format!("DOT — Pixel & Icon Studio — {file_name} ({})", path.display())
            }
            None => "DOT — Pixel & Icon Studio — Untitled".to_string(),
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));


        // Scurtături tastatură fișiere (Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S, Ctrl+W)
        let mut trigger_new = false;
        let mut trigger_open = false;
        let mut trigger_save = false;
        let mut trigger_save_as = false;
        let mut trigger_close = false;

        let mut trigger_undo = false;
        let mut trigger_redo = false;
        let mut trigger_cut = false;
        let mut trigger_copy = false;
        let mut trigger_paste = false;

        ctx.input_mut(|i| {
            if i.modifiers.ctrl && !i.modifiers.alt {
                if i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S) {
                    trigger_save_as = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                    trigger_save = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::N) {
                    trigger_new = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::O) {
                    trigger_open = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::W) {
                    trigger_close = true;
                }
            }
        });

        if trigger_new {
            *self = Self::default();
        }
        if trigger_open {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Windows Icon", &["ico"])
                .pick_file()
            {
                self.open_ico(&path);
            }
        }
        if trigger_save {
            self.save_current_ico();
        }
        if trigger_save_as {
            self.save_ico_as();
        }
        if trigger_close {
            self.close_file();
        }


        // Scurtături tastatură (Ctrl+Z / Ctrl+Y)
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.frames[self.active_frame].undo();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
            self.frames[self.active_frame].redo();
        }

        ctx.input_mut(|i| {
            if i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.shift {
                if i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) {
                    trigger_undo = true;
                } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) {
                    trigger_redo = true;
                } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::X) {
                    trigger_cut = true;
                } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::C) {
                    trigger_copy = true;
                } else if i.consume_key(egui::Modifiers::CTRL, egui::Key::V) {
                    trigger_paste = true;
                }
            }
        });

        if trigger_undo {
            self.edit_undo();
        }
        if trigger_redo {
            self.edit_redo();
        }
        if trigger_cut {
            self.edit_cut();
        }
        if trigger_copy {
            self.edit_copy();
        }
        if trigger_paste {
            self.edit_paste();
        }

        let mut trigger_select_all = false;
        let mut trigger_deselect = false;
        let mut trigger_clear_sel = false;

        ctx.input_mut(|i| {
            if i.key_pressed(egui::Key::Escape) {
                trigger_deselect = true;
            }
            if i.key_pressed(egui::Key::Delete) {
                trigger_clear_sel = true;
            }
            if i.modifiers.ctrl && !i.modifiers.alt && !i.modifiers.shift {
                if i.consume_key(egui::Modifiers::CTRL, egui::Key::A) {
                    trigger_select_all = true;
                }
            }
        });

        if trigger_select_all {
            self.select_all();
        }
        if trigger_deselect {
            self.deselect();
        }
        if trigger_clear_sel {
            self.clear_selection_pixels();
        }


        // Actualizăm texturile pentru preview
        for (idx, f) in self.frames.iter_mut().enumerate() {
            f.update_texture(ctx, idx);
        }

        // 1. BARA DE MENIU SUS
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                //ui.heading("DOT");
                //ui.separator();

                ui.menu_button("File", |ui| {
                    if ui.add(egui::Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                        *self = Self::default();
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
                    if ui.add(egui::Button::new("Save As").shortcut_text("Ctrl+Shift+S")).clicked() {
                        self.save_ico_as();
                        ui.close_menu();
                    }
                    if ui.add(egui::Button::new("Close").shortcut_text("Ctrl+W")).clicked() {
                        self.close_file();
                        ui.close_menu();
                    }

                    ui.separator();

                    ui.menu_button("Import", |ui| {
                        if ui.add(egui::Button::new("Image (PNG / JPG / BMP)...").shortcut_text("Ctrl+I")).clicked() {
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
                        if ui.add(egui::Button::new("Active Frame as PNG...").shortcut_text("Ctrl+E")).clicked() {
                            self.export_active_png();
                            ui.close_menu();
                        }
                        if ui.add(egui::Button::new("Windows ICO Set (32/8/4-bit)...").shortcut_text("Ctrl+Shift+E")).clicked() {
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

            });
        });

        // 2. BARA DE STARE JOS
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
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
        });

        // 3. PANOU STÂNGA: UNELTE ȘI CULORI
        egui::SidePanel::left("tools_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Unelte");
                ui.horizontal(|ui| {
                    if tool_button(ui, self.tool == Tool::Pencil, "Creion (Desen)", draw_pencil_icon) {
                        self.tool = Tool::Pencil;
                    }
                    if tool_button(ui, self.tool == Tool::Eraser, "Radiera (Transparent)", draw_eraser_icon) {
                        self.tool = Tool::Eraser;
                    }
                    if tool_button(ui, self.tool == Tool::Fill, "Galeata (Flood Fill)", draw_fill_icon) {
                        self.tool = Tool::Fill;
                    }
                    if tool_button(ui, self.tool == Tool::Eyedropper, "Pipeta (Alege culoare)", draw_eyedropper_icon) {
                        self.tool = Tool::Eyedropper;
                    }
                    if tool_button(ui, self.tool == Tool::SelectRect, "Selectie Dreptunghi", draw_select_rect_icon) {
                        self.tool = Tool::SelectRect;
                    }
                });

                

                ui.separator();
                ui.heading("Culoare");
                let mut srgba = egui::Color32::from_rgba_unmultiplied(
                    self.primary_color[0],
                    self.primary_color[1],
                    self.primary_color[2],
                    self.primary_color[3],
                );
                if egui::color_picker::color_picker_color32(
                    ui,
                    &mut srgba,
                    egui::color_picker::Alpha::OnlyBlend,
                ) {
                    self.primary_color = srgba.to_array();
                }

                ui.separator();
                ui.heading("Actiuni Frame");
                ui.horizontal(|ui| {
                    if ui.button("Undo").clicked() {
                        self.frames[self.active_frame].undo();
                    }
                    if ui.button("Redo").clicked() {
                        self.frames[self.active_frame].redo();
                    }
                });
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
                ui.heading("Cuantizare & Afisare");
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

        // 4. PANOU DREAPTA: REZOLUȚII & ADÂNCIMI DE CULOARE .ICO
        egui::SidePanel::right("frames_panel")
            .min_width(240.0)
            .show(ctx, |ui| {
                ui.heading("Formate in .ICO");
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
                                ui.selectable_value(
                                    &mut self.new_frame_size,
                                    s,
                                    format!("{s}x{s}"),
                                );
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
            });

        // 4b. CONSOLĂ LIMBAJ DOT (MULTI-LINIE + VARIABILE PERSISTENTE + LOAD & RUN)
        egui::TopBottomPanel::bottom("command_panel")
            .resizable(true)
            .min_height(135.0_f32)
            .default_height(185.0_f32)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Console (dot)");
                    ui.label("(Ctrl+Enter sau Run)");
                    if ui.small_button("Clear").clicked() {
                        self.cmd_history.clear();
                    }
                    if ui.small_button("Reset Vars").clicked() {
                        self.cmd_vars.clear();
                        self.cmd_history.push("[variabile resetate]".to_string());
                    }

                    ui.separator();
                    ui.label("Fișier:");
                    let path_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.dot_script_path)
                            .hint_text("ex: scripts/fractal_tree.dot")
                            .desired_width(210.0_f32),
                    );

                    if ui.small_button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Script DOT", &["dot"])
                            .pick_file()
                        {
                            self.dot_script_path = path.display().to_string();
                            self.load_dot_script_from_path(true);
                        }
                    }

                    if ui.small_button("Load").clicked() {
                        self.load_dot_script_from_path(false);
                    }

                    let enter_on_path = path_resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.small_button("Load & Run").clicked() || enter_on_path {
                        self.load_dot_script_from_path(true);
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(80.0_f32)
                    .show(ui, |ui| {
                        for entry in &self.cmd_history {
                            ui.monospace(entry);
                        }
                    });

                let open_braces = self
                    .cmd_input
                    .chars()
                    .fold(0_i32, |acc, c| match c {
                        '{' => acc + 1,
                        '}' => acc - 1,
                        _ => acc,
                    });

                let ctrl_enter = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter));
                let plain_enter_single_line = !self.cmd_input.contains('\n')
                    && open_braces <= 0
                    && ui.input(|i| {
                        !i.modifiers.ctrl
                            && !i.modifiers.shift
                            && i.key_pressed(egui::Key::Enter)
                    });

                ui.horizontal_top(|ui| {
                    ui.monospace(if open_braces > 0 { ".." } else { ">>" });
                    let edit = ui.add(
                        egui::TextEdit::multiline(&mut self.cmd_input)
                            .desired_rows(3)
                            .desired_width((ui.available_width() - 72.0_f32).max(120.0_f32))
                            .code_editor()
                            .hint_text(
                                "ex:\nlet c = #ff0000\nif get_pixel(10, 10) == c {\n  put_pixel(mix(c, #00ff00, 0.5), 10, 10)\n}",
                            ),
                    );

                    let mut run_clicked = ui.button("Run").clicked();
                    if ctrl_enter || plain_enter_single_line {
                        run_clicked = true;
                    }

                    if run_clicked {
                        let script = std::mem::take(&mut self.cmd_input);
                        self.run_dot_command(&script);
                        edit.request_focus();
                    }
                });
            });

        // 5. ZONA CENTRALĂ: CANVAS DE EDITARE PIXEL-CU-PIXEL (OPTIMIZAT GPU)
        let checker_tex_id = self.get_checker_texture(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Zoom cu Ctrl + Scroll
            let scroll_delta_y = ctx.input(|i| {
                if i.modifiers.ctrl {
                    i.smooth_scroll_delta.y
                } else {
                    0.0
                }
            });
            if scroll_delta_y.abs() > 0.0 {
                let factor = if scroll_delta_y > 0.0 { 1.12 } else { 0.89 };
                self.zoom = (self.zoom * factor).clamp(2.0, 32.0);
            }

            egui::ScrollArea::both()
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    let frame = &mut self.frames[self.active_frame];
                    let canvas_w = frame.width as f32 * self.zoom;
                    let canvas_h = frame.height as f32 * self.zoom;

                    // Centrăm canvas-ul dacă spațiul disponibil este mai mare decât el
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

                    // 1. Fundalul șah (1 singur dreptunghi GPU repetat)
                    let uv_checker = egui::Rect::from_min_max(
                        egui::pos2(0.0, 0.0),
                        egui::pos2(frame.width as f32 / 2.0, frame.height as f32 / 2.0),
                    );
                    painter.image(checker_tex_id, canvas_rect, uv_checker, egui::Color32::WHITE);

                    // 2. Stratul de pixeli al iconiței (1 singură textură NEAREST)
                    if let Some(tex) = &frame.texture {
                        painter.image(
                            tex.id(),
                            canvas_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    // 3. Liniile de grid (doar cele vizibile în viewport)
                    if self.show_grid && self.zoom >= 6.0 {
                        let grid_stroke = egui::Stroke::new(0.5_f32, egui::Color32::from_gray(75));
                        let clip = painter.clip_rect();

                        let start_x = (((clip.min.x - canvas_rect.min.x) / self.zoom).floor()
                            as i32)
                            .max(0) as u32;
                        let end_x = (((clip.max.x - canvas_rect.min.x) / self.zoom).ceil() as i32)
                            .min(frame.width as i32)
                            .max(0) as u32;

                        let start_y = (((clip.min.y - canvas_rect.min.y) / self.zoom).floor()
                            as i32)
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

                    // Chenar discret în jurul iconiței
                    painter.rect_stroke(
                        canvas_rect.expand(1.0_f32),
                        0.0,
                        egui::Stroke::new(1.0_f32, egui::Color32::from_gray(95)),
                    );

                    // Desenăm chenarul de selecție dacă există o selecție activă
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
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 190)),
                        );
                        painter.rect_stroke(
                            sel_rect.expand(1.0_f32),
                            0.0,
                            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 59, 48)),
                        );
                    }

                    // Coordonata pixelului de sub cursor (pentru hover & desen)
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

                            // Evidențiem pixelul de sub cursor
                            let px_rect = egui::Rect::from_min_size(
                                egui::pos2(
                                    canvas_rect.min.x + gx as f32 * self.zoom,
                                    canvas_rect.min.y + gy as f32 * self.zoom,
                                ),
                                egui::vec2(self.zoom, self.zoom),
                            );
                            painter.rect_stroke(
                                px_rect,
                                0.0,
                                egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
                            );

                            // Pregătire început drag
                            if response.drag_started() {
                                if matches!(self.tool, Tool::Pencil | Tool::Eraser) {
                                    frame.push_undo();
                                    self.last_draw_pos = Some((gx, gy));
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
                                        let min_x =
                                            start.0.min(gx).clamp(0, frame.width as i32 - 1) as u32;
                                        let min_y =
                                            start.1.min(gy).clamp(0, frame.height as i32 - 1) as u32;
                                        let max_x =
                                            start.0.max(gx).clamp(0, frame.width as i32 - 1) as u32;
                                        let max_y =
                                            start.1.max(gy).clamp(0, frame.height as i32 - 1) as u32;
                                        self.selection = Some((min_x, min_y, max_x, max_y));
                                    }
                                    Tool::Pencil | Tool::Eraser => {
                                        let draw_col = if self.tool == Tool::Pencil {
                                            self.primary_color
                                        } else {
                                            [0, 0, 0, 0]
                                        };
                                        let (sx, sy) = self.last_draw_pos.unwrap_or((gx, gy));
                                        plot_line(sx, sy, gx, gy, |px, py| {
                                            if px >= 0
                                                && px < frame.width as i32
                                                && py >= 0
                                                && py < frame.height as i32
                                            {
                                                frame.set_pixel(px as u32, py as u32, draw_col);
                                            }
                                        });
                                        self.last_draw_pos = Some((gx, gy));
                                    }
                                    Tool::Fill => {
                                        if response.clicked() {
                                            frame.push_undo();
                                            frame.flood_fill(
                                                gx as u32,
                                                gy as u32,
                                                self.primary_color,
                                            );
                                        }
                                    }
                                    Tool::Eyedropper => {
                                        self.primary_color = frame.get_pixel(gx as u32, gy as u32);
                                        self.tool = Tool::Pencil;
                                    }
                                }
                            }
                        }
                    }

                    if response.drag_stopped() {
                        self.last_draw_pos = None;
                        self.select_drag_start = None;
                    }
                });
        });
    }
}



fn main() -> eframe::Result<()> {
    let startup_path = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DOT — Pixel & Icon Studio")
            .with_inner_size([1180.0, 740.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DOT",
        options,
        Box::new(move |_cc| Ok(Box::new(IconEditorApp::from_startup_path(startup_path)))),
    )
}