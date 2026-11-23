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
    checker_texture: Option<egui::TextureHandle>,
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
            checker_texture: None,
        }
    }
}

impl IconEditorApp {
    fn from_startup_path(path: Option<PathBuf>) -> Self {
        let mut app = Self::default();
        if let Some(p) = path {
            if p.is_file() {
                match p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase())
                    .as_deref()
                {
                    Some("ico") => app.open_ico(&p),
                    Some("png" | "jpg" | "jpeg" | "bmp") => app.import_png(&p),
                    _ => app.status_msg = format!("Format nesuportat la pornire: {:?}", p),
                }
            } else if p.is_dir() {
                app.status_msg = format!("Director proiect: {:?}", p);
            }
        }
        app
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
        // Scurtături tastatură (Ctrl+Z / Ctrl+Y)
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            self.frames[self.active_frame].undo();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) {
            self.frames[self.active_frame].redo();
        }

        // Actualizăm texturile pentru preview
        for (idx, f) in self.frames.iter_mut().enumerate() {
            f.update_texture(ctx, idx);
        }

        // 1. BARA DE MENIU SUS
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("DOT");
                ui.separator();

                if ui.button("Nou").clicked() {
                    *self = Self::default();
                }
                if ui.button("Importa PNG").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagini", &["png", "jpg", "bmp"])
                        .pick_file()
                    {
                        self.import_png(&path);
                    }
                }
                if ui.button("Deschide .ICO").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Windows Icon", &["ico"])
                        .pick_file()
                    {
                        self.open_ico(&path);
                    }
                }
                if ui.button("Salveaza .ICO").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("icon.ico")
                        .add_filter("Windows Icon", &["ico"])
                        .save_file()
                    {
                        self.save_ico(&path);
                    }
                }
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
            });
        });

        // 3. PANOU STÂNGA: UNELTE ȘI CULORI
        egui::SidePanel::left("tools_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Unelte");
                ui.selectable_value(&mut self.tool, Tool::Pencil, "Creion (Desen)");
                ui.selectable_value(&mut self.tool, Tool::Eraser, "Radiera (Transparent)");
                ui.selectable_value(&mut self.tool, Tool::Fill, "Galeata (Flood Fill)");
                ui.selectable_value(&mut self.tool, Tool::Eyedropper, "Pipeta (Alege culoare)");

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

        // 5. ZONA CENTRALĂ: CANVAS DE EDITARE PIXEL-CU-PIXEL (OPTIMIZAT GPU)
        let checker_tex_id = self.get_checker_texture(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                let frame = &mut self.frames[self.active_frame];
                let canvas_w = frame.width as f32 * self.zoom;
                let canvas_h = frame.height as f32 * self.zoom;

                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(canvas_w, canvas_h),
                    egui::Sense::click_and_drag(),
                );

                let painter = ui.painter_at(rect);

                // 1. Fundalul șah (1 singur dreptunghi GPU repetat)
                let uv_checker = egui::Rect::from_min_max(
                    egui::pos2(0.0, 0.0),
                    egui::pos2(frame.width as f32 / 2.0, frame.height as f32 / 2.0),
                );
                painter.image(checker_tex_id, rect, uv_checker, egui::Color32::WHITE);

                // 2. Stratul de pixeli al iconiței (1 singură textură NEAREST)
                if let Some(tex) = &frame.texture {
                    painter.image(
                        tex.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                // 3. Liniile de grid (doar cele vizibile în viewport)
                if self.show_grid && self.zoom >= 6.0 {
                    let grid_stroke = egui::Stroke::new(0.5_f32, egui::Color32::from_gray(75));
                    let clip = painter.clip_rect();

                    let start_x =
                        (((clip.min.x - rect.min.x) / self.zoom).floor() as i32).max(0) as u32;
                    let end_x = (((clip.max.x - rect.min.x) / self.zoom).ceil() as i32)
                        .min(frame.width as i32)
                        .max(0) as u32;

                    let start_y =
                        (((clip.min.y - rect.min.y) / self.zoom).floor() as i32).max(0) as u32;
                    let end_y = (((clip.max.y - rect.min.y) / self.zoom).ceil() as i32)
                        .min(frame.height as i32)
                        .max(0) as u32;

                    let y_min = clip.min.y.max(rect.min.y);
                    let y_max = clip.max.y.min(rect.max.y);
                    for x in start_x..=end_x {
                        let px = rect.min.x + x as f32 * self.zoom;
                        painter
                            .line_segment([egui::pos2(px, y_min), egui::pos2(px, y_max)], grid_stroke);
                    }

                    let x_min = clip.min.x.max(rect.min.x);
                    let x_max = clip.max.x.min(rect.max.x);
                    for y in start_y..=end_y {
                        let py = rect.min.y + y as f32 * self.zoom;
                        painter
                            .line_segment([egui::pos2(x_min, py), egui::pos2(x_max, py)], grid_stroke);
                    }
                }

                // Interacțiune mouse pe canvas
                if response.drag_started() {
                    if matches!(self.tool, Tool::Pencil | Tool::Eraser) {
                        frame.push_undo();
                    }
                }

                if response.clicked() || response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let gx = ((pos.x - rect.min.x) / self.zoom).floor() as i32;
                        let gy = ((pos.y - rect.min.y) / self.zoom).floor() as i32;

                        if gx >= 0 && gx < frame.width as i32 && gy >= 0 && gy < frame.height as i32
                        {
                            match self.tool {
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
                                        frame.flood_fill(gx as u32, gy as u32, self.primary_color);
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