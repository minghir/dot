use eframe::egui;
use image::codecs::png::PngEncoder;
use image::ImageEncoder;

pub(crate) const STANDARD_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum ColorDepth {
    Bit1,
    Bit4,
    Bit8,
    Bit24,
    Bit32,
}

impl ColorDepth {
    pub(crate) const ALL: [ColorDepth; 5] = [
        ColorDepth::Bit32,
        ColorDepth::Bit24,
        ColorDepth::Bit8,
        ColorDepth::Bit4,
        ColorDepth::Bit1,
    ];

    pub(crate) fn bits(self) -> u16 {
        match self {
            ColorDepth::Bit1 => 1,
            ColorDepth::Bit4 => 4,
            ColorDepth::Bit8 => 8,
            ColorDepth::Bit24 => 24,
            ColorDepth::Bit32 => 32,
        }
    }

    pub(crate) fn short_label(self) -> &'static str {
        match self {
            ColorDepth::Bit1 => "1-bit",
            ColorDepth::Bit4 => "4-bit",
            ColorDepth::Bit8 => "8-bit",
            ColorDepth::Bit24 => "24-bit",
            ColorDepth::Bit32 => "32-bit",
        }
    }

    pub(crate) fn full_label(self) -> &'static str {
        match self {
            ColorDepth::Bit1 => "1-bit (2 culori)",
            ColorDepth::Bit4 => "4-bit (16 culori)",
            ColorDepth::Bit8 => "8-bit (256 culori)",
            ColorDepth::Bit24 => "24-bit (RGB + masca)",
            ColorDepth::Bit32 => "32-bit (RGBA Alpha)",
        }
    }

    pub(crate) fn from_bits(bits: u16) -> Self {
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
pub(crate) enum Tool {
    Pencil,
    Eraser,
    Fill,
    Eyedropper,
    SelectRect,
}

#[derive(Clone, Debug)]
pub(crate) struct Layer {
    pub(crate) name: String,
    pub(crate) visible: bool,
    pub(crate) opacity: f32, // 0.0 ..= 1.0
    pub(crate) pixels: Vec<[u8; 4]>,
}

impl Layer {
    pub(crate) fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        let count = (width * height) as usize;
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            pixels: vec![[0, 0, 0, 0]; count],
        }
    }

    pub(crate) fn from_pixels(name: impl Into<String>, pixels: Vec<[u8; 4]>) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            pixels,
        }
    }
}

#[derive(Clone)]
pub(crate) struct HistorySnapshot {
    pub(crate) layers: Vec<Layer>,
    pub(crate) active_layer: usize,
}


pub(crate) struct IconFrame {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) color_depth: ColorDepth,
    pub(crate) layers: Vec<Layer>,
    pub(crate) active_layer: usize,
    pub(crate) pixels: Vec<[u8; 4]>, // Buffer compozit final RGBA8
    pub(crate) undo_stack: Vec<HistorySnapshot>,
    pub(crate) redo_stack: Vec<HistorySnapshot>,
    pub(crate) texture: Option<egui::TextureHandle>,
    pub(crate) dirty: bool,
}

impl IconFrame {
    pub(crate) fn new(width: u32, height: u32, color_depth: ColorDepth) -> Self {
        let count = (width * height) as usize;
        Self {
            width,
            height,
            color_depth,
            layers: vec![Layer::new("Layer 0", width, height)],
            active_layer: 0,
            pixels: vec![[0, 0, 0, 0]; count],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            texture: None,
            dirty: true,
        }
    }

    pub(crate) fn from_rgba(
        width: u32,
        height: u32,
        color_depth: ColorDepth,
        raw: Vec<u8>,
    ) -> Self {
        let pixels: Vec<[u8; 4]> = raw
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();
        Self {
            width,
            height,
            color_depth,
            layers: vec![Layer::from_pixels("Layer 0", pixels.clone())],
            active_layer: 0,
            pixels,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            texture: None,
            dirty: true,
        }
    }

    pub(crate) fn active_layer_mut(&mut self) -> &mut Layer {
        if self.layers.is_empty() {
            self.layers.push(Layer::new("Layer 0", self.width, self.height));
            self.active_layer = 0;
        }
        let idx = self.active_layer.min(self.layers.len() - 1);
        self.active_layer = idx;
        &mut self.layers[idx]
    }

    /// Compune toate layerele vizibile (de jos în sus: index 0 -> len-1) în `self.pixels`
    pub(crate) fn recompose(&mut self) {
        let count = (self.width * self.height) as usize;
        self.pixels.fill([0, 0, 0, 0]);

        for layer in &self.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            let layer_op = layer.opacity.clamp(0.0, 1.0);

            for i in 0..count.min(layer.pixels.len()) {
                let src = layer.pixels[i];
                if src[3] == 0 {
                    continue;
                }

                let sa = (src[3] as f32 / 255.0) * layer_op;
                if sa <= 0.0 {
                    continue;
                }

                let dst = self.pixels[i];
                let da = dst[3] as f32 / 255.0;
                let out_a = sa + da * (1.0 - sa);

                if out_a <= 0.0 {
                    self.pixels[i] = [0, 0, 0, 0];
                } else {
                    let blend = |sc: u8, dc: u8| -> u8 {
                        let c = ((sc as f32 * sa) + (dc as f32 * da * (1.0 - sa))) / out_a;
                        c.round().clamp(0.0, 255.0) as u8
                    };
                    let mut final_a = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
                    if self.color_depth != ColorDepth::Bit32 {
                        final_a = if final_a < 128 { 0 } else { 255 };
                    }
                    self.pixels[i] = if final_a == 0 {
                        [0, 0, 0, 0]
                    } else {
                        [
                            blend(src[0], dst[0]),
                            blend(src[1], dst[1]),
                            blend(src[2], dst[2]),
                            final_a,
                        ]
                    };
                }
            }
        }
        self.dirty = true;
    }

    pub(crate) fn add_layer(&mut self, name: Option<String>) {
        self.push_undo();
        let next_num = self.layers.len();
        let layer_name = name.unwrap_or_else(|| format!("Layer {next_num}"));
        let insert_idx = (self.active_layer + 1).min(self.layers.len());
        self.layers
            .insert(insert_idx, Layer::new(layer_name, self.width, self.height));
        self.active_layer = insert_idx;
        self.recompose();
    }

    pub(crate) fn duplicate_layer(&mut self, idx: usize) {
        if idx >= self.layers.len() {
            return;
        }
        self.push_undo();
        let mut copy = self.layers[idx].clone();
        copy.name = format!("{} (copy)", copy.name);
        let insert_idx = idx + 1;
        self.layers.insert(insert_idx, copy);
        self.active_layer = insert_idx;
        self.recompose();
    }

    pub(crate) fn remove_layer(&mut self, idx: usize) {
        if self.layers.len() <= 1 || idx >= self.layers.len() {
            return;
        }
        self.push_undo();
        self.layers.remove(idx);
        if self.active_layer >= self.layers.len() {
            self.active_layer = self.layers.len() - 1;
        }
        self.recompose();
    }

    pub(crate) fn move_layer(&mut self, from: usize, to: usize) {
        if from >= self.layers.len() || to >= self.layers.len() || from == to {
            return;
        }
        self.push_undo();
        let layer = self.layers.remove(from);
        self.layers.insert(to, layer);
        self.active_layer = to;
        self.recompose();
    }

    pub(crate) fn merge_down(&mut self, idx: usize) {
        if idx == 0 || idx >= self.layers.len() {
            return;
        }
        self.push_undo();
        let top = self.layers.remove(idx);
        let bottom = &mut self.layers[idx - 1];
        let top_op = if top.visible {
            top.opacity.clamp(0.0, 1.0)
        } else {
            0.0
        };

        for i in 0..bottom.pixels.len().min(top.pixels.len()) {
            let src = top.pixels[i];
            let sa = (src[3] as f32 / 255.0) * top_op;
            if sa <= 0.0 {
                continue;
            }
            let dst = bottom.pixels[i];
            let da = dst[3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                bottom.pixels[i] = [0, 0, 0, 0];
            } else {
                let blend = |sc: u8, dc: u8| -> u8 {
                    let c = ((sc as f32 * sa) + (dc as f32 * da * (1.0 - sa))) / out_a;
                    c.round().clamp(0.0, 255.0) as u8
                };
                bottom.pixels[i] = [
                    blend(src[0], dst[0]),
                    blend(src[1], dst[1]),
                    blend(src[2], dst[2]),
                    (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
                ];
            }
        }

        self.active_layer = idx - 1;
        self.recompose();
    }

    pub(crate) fn flatten_layers(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        self.push_undo();
        self.recompose();
        self.layers = vec![Layer::from_pixels("Background", self.pixels.clone())];
        self.active_layer = 0;
        self.dirty = true;
    }

    pub(crate) fn push_undo(&mut self) {
        if self.undo_stack.len() >= 30 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(HistorySnapshot {
            layers: self.layers.clone(),
            active_layer: self.active_layer,
        });
        self.redo_stack.clear();
    }

    pub(crate) fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(HistorySnapshot {
                layers: self.layers.clone(),
                active_layer: self.active_layer,
            });
            self.layers = prev.layers;
            self.active_layer = prev.active_layer.min(self.layers.len().saturating_sub(1));
            self.recompose();
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(HistorySnapshot {
                layers: self.layers.clone(),
                active_layer: self.active_layer,
            });
            self.layers = next.layers;
            self.active_layer = next.active_layer.min(self.layers.len().saturating_sub(1));
            self.recompose();
        }
    }

    /// Citește pixelul compozit (din toate layerele vizibile)
    pub(crate) fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        self.pixels[(y * self.width + x) as usize]
    }

    /// Citește pixelul exclusiv de pe layer-ul activ
    pub(crate) fn get_active_layer_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let idx = self.active_layer.min(self.layers.len() - 1);
        self.layers[idx].pixels[(y * self.width + x) as usize]
    }

    /// Scrie un pixel pe layer-ul activ (necesită `recompose()` la sfârșitul operației)
    pub(crate) fn set_pixel_raw(&mut self, x: u32, y: u32, mut color: [u8; 4]) {
        if self.color_depth != ColorDepth::Bit32 {
            color[3] = if color[3] < 128 { 0 } else { 255 };
        }
        let idx = (y * self.width + x) as usize;
        let layer = self.active_layer_mut();
        if layer.pixels[idx] != color {
            layer.pixels[idx] = color;
        }
    }

    pub(crate) fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        self.set_pixel_raw(x, y, color);
        self.recompose();
    }

    pub(crate) fn clear_active_layer(&mut self) {
        self.active_layer_mut().pixels.fill([0, 0, 0, 0]);
        self.recompose();
    }

    pub(crate) fn remove_corner_background(&mut self, tolerance: f32) {
        let w = self.width as usize;
        let h = self.height as usize;
        if w == 0 || h == 0 {
            return;
        }
        let bg = self.get_active_layer_pixel(0, 0);
        let mut visited = vec![false; w * h];
        let mut stack = vec![
            (0u32, 0u32),
            (self.width - 1, 0),
            (0, self.height - 1),
            (self.width - 1, self.height - 1),
        ];
        let feather = 30.0_f32;
        let width = self.width;
        let height = self.height;
        let layer = self.active_layer_mut();

        while let Some((x, y)) = stack.pop() {
            let idx = y as usize * w + x as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            let p = layer.pixels[idx];
            if p[3] == 0 {
                continue;
            }

            let dr = p[0] as f32 - bg[0] as f32;
            let dg = p[1] as f32 - bg[1] as f32;
            let db = p[2] as f32 - bg[2] as f32;
            let dist = (dr * dr + dg * dg + db * db).sqrt();

            if dist <= tolerance {
                layer.pixels[idx] = [0, 0, 0, 0];
                if x > 0 {
                    stack.push((x - 1, y));
                }
                if x + 1 < width {
                    stack.push((x + 1, y));
                }
                if y > 0 {
                    stack.push((x, y - 1));
                }
                if y + 1 < height {
                    stack.push((x, y + 1));
                }
            } else if dist < tolerance + feather {
                let alpha = ((dist - tolerance) / feather).clamp(0.0, 1.0);
                let unblend = |c: u8, b: u8| -> u8 {
                    (((c as f32 - (1.0 - alpha) * b as f32) / alpha).clamp(0.0, 255.0)) as u8
                };
                layer.pixels[idx] = [
                    unblend(p[0], bg[0]),
                    unblend(p[1], bg[1]),
                    unblend(p[2], bg[2]),
                    (alpha * 255.0) as u8,
                ];
            }
        }
        self.recompose();
    }

    pub(crate) fn flood_fill_with_options(
        &mut self,
        start_x: u32,
        start_y: u32,
        mut new_color: [u8; 4],
        tolerance: u8,
        contiguous: bool,
    ) {
        if self.color_depth != ColorDepth::Bit32 {
            new_color[3] = if new_color[3] < 128 { 0 } else { 255 };
        }
        let target = self.get_active_layer_pixel(start_x, start_y);
        if tolerance == 0 && target == new_color {
            return;
        }

        let tol_i32 = tolerance as i32;
        let matches = |px: [u8; 4]| -> bool {
            if tolerance == 0 {
                return px == target;
            }
            (px[0] as i32 - target[0] as i32).abs() <= tol_i32
                && (px[1] as i32 - target[1] as i32).abs() <= tol_i32
                && (px[2] as i32 - target[2] as i32).abs() <= tol_i32
                && (px[3] as i32 - target[3] as i32).abs() <= tol_i32
        };

        if !contiguous {
            for y in 0..self.height {
                for x in 0..self.width {
                    if matches(self.get_active_layer_pixel(x, y)) {
                        self.set_pixel_raw(x, y, new_color);
                    }
                }
            }
            self.recompose();
            return;
        }

        let w = self.width as usize;
        let h = self.height as usize;
        let mut visited = vec![false; w * h];
        let mut stack = vec![(start_x, start_y)];

        while let Some((x, y)) = stack.pop() {
            let idx = y as usize * w + x as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            if !matches(self.get_active_layer_pixel(x, y)) {
                continue;
            }

            self.set_pixel_raw(x, y, new_color);

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
        self.recompose();
    }

    pub(crate) fn apply_color_depth(&mut self, dithering: bool) {
        self.recompose();
        match self.color_depth {
            ColorDepth::Bit32 => {}
            ColorDepth::Bit24 => {
                for layer in &mut self.layers {
                    for p in &mut layer.pixels {
                        if p[3] < 128 {
                            *p = [0, 0, 0, 0];
                        } else {
                            p[3] = 255;
                        }
                    }
                }
                self.recompose();
            }
            ColorDepth::Bit8 | ColorDepth::Bit4 | ColorDepth::Bit1 => {
                let max_colors = 1usize << self.color_depth.bits();
                let palette = build_palette(&self.pixels, max_colors);
                let (quantized, _) =
                    quantize_pixels(&self.pixels, self.width, self.height, &palette, dithering);
                self.layers = vec![Layer::from_pixels("Indexed", quantized)];
                self.active_layer = 0;
                self.recompose();
            }
        }
    }

    pub(crate) fn to_flat_rgba(&self) -> Vec<u8> {
        let mut flat = Vec::with_capacity(self.pixels.len() * 4);
        for p in &self.pixels {
            flat.extend_from_slice(p);
        }
        flat
    }

    pub(crate) fn update_texture(&mut self, ctx: &egui::Context, index: usize) {
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

    pub(crate) fn encode_ico_payload(&self, dithering: bool) -> Result<Vec<u8>, String> {
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

        buf.extend_from_slice(&40u32.to_le_bytes());
        buf.extend_from_slice(&(self.width as i32).to_le_bytes());
        buf.extend_from_slice(&((self.height * 2) as i32).to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&bits.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&((xor_size + and_size) as u32).to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());
        let clr_used = if palette.is_empty() {
            0u32
        } else {
            palette.len() as u32
        };
        buf.extend_from_slice(&clr_used.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());

        for [r, g, b] in &palette {
            buf.extend_from_slice(&[*b, *g, *r, 0]);
        }

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
    palette.push([0, 0, 0]);

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

pub(crate) fn plot_line(x0: i32, y0: i32, x1: i32, y1: i32, mut plot: impl FnMut(i32, i32)) {
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