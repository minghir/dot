#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod dock;
mod model;
mod panels;
mod tools;

use dock::{create_default_dock_state, EditorTabViewer, PanelTab};
use eframe::egui;
use egui_dock::{DockArea, DockState, Style};
use model::{ColorDepth, IconFrame, Tool, STANDARD_SIZES};
use std::path::PathBuf;

pub(crate) struct IconEditorApp {
    pub(crate) dock_state: DockState<PanelTab>,
    pub(crate) frames: Vec<IconFrame>,
    pub(crate) active_frame: usize,
    pub(crate) tool: Tool,
    pub(crate) primary_color: [u8; 4],
    pub(crate) zoom: f32,
    pub(crate) show_grid: bool,
    pub(crate) use_dithering: bool,
    pub(crate) new_frame_size: u32,
    pub(crate) new_frame_depth: ColorDepth,
    pub(crate) last_draw_pos: Option<(i32, i32)>,
    pub(crate) status_msg: String,
    pub(crate) current_file: Option<PathBuf>,
    pub(crate) clipboard: Option<(u32, u32, Vec<[u8; 4]>)>,
    pub(crate) checker_texture: Option<egui::TextureHandle>,
    pub(crate) selection: Option<(u32, u32, u32, u32)>,
    pub(crate) select_drag_start: Option<(i32, i32)>,
    pub(crate) hovered_coord: Option<(u32, u32)>,
    pub(crate) cmd_input: String,
    pub(crate) cmd_history: Vec<String>,
    pub(crate) cmd_vars: std::collections::HashMap<String, dot_lang::Value>,
    pub(crate) cmd_funcs: std::collections::HashMap<String, dot_lang::UserFn>,
    pub(crate) cmd_structs: std::collections::HashMap<String, dot_lang::StructDef>,
    pub(crate) dot_script_path: String,
    pub(crate) tool_settings: tools::ToolSettings,
    pub(crate) stroke_points: Vec<(i32, i32)>,
    pub(crate) renaming_layer: Option<usize>,
}

impl Default for IconEditorApp {
    fn default() -> Self {
        let frames = STANDARD_SIZES
            .iter()
            .map(|&s| IconFrame::new(s, s, ColorDepth::Bit32))
            .collect();
        Self {
            dock_state: create_default_dock_state(),
            frames,
            active_frame: 2,
            tool: Tool::Pencil,
            primary_color: [255, 59, 48, 255],
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
            tool_settings: tools::ToolSettings::default(),
            stroke_points: Vec::new(),
            renaming_layer: None,
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
        let mut trigger_select_all = false;
        let mut trigger_deselect = false;
        let mut trigger_clear_sel = false;

        ctx.input_mut(|i| {
            if i.modifiers.ctrl && !i.modifiers.alt {
                if i.modifiers.shift
                    && i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)
                {
                    trigger_save_as = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::S) {
                    trigger_save = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::N) {
                    trigger_new = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::O) {
                    trigger_open = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::W) {
                    trigger_close = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::Z) {
                    trigger_undo = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::Y) {
                    trigger_redo = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::X) {
                    trigger_cut = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::C) {
                    trigger_copy = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::V) {
                    trigger_paste = true;
                } else if !i.modifiers.shift && i.consume_key(egui::Modifiers::CTRL, egui::Key::A) {
                    trigger_select_all = true;
                }
            }
            if i.key_pressed(egui::Key::Escape) {
                trigger_deselect = true;
            }
            if i.key_pressed(egui::Key::Delete) {
                trigger_clear_sel = true;
            }
        });

        if trigger_new {
            let preserved_dock =
                std::mem::replace(&mut self.dock_state, DockState::new(vec![]));
            *self = Self::default();
            self.dock_state = preserved_dock;
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
        if trigger_select_all {
            self.select_all();
        }
        if trigger_deselect {
            self.deselect();
        }
        if trigger_clear_sel {
            self.clear_selection_pixels();
        }

        for (idx, f) in self.frames.iter_mut().enumerate() {
            f.update_texture(ctx, idx);
        }

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            self.show_top_bar(ctx, ui);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            self.show_status_bar(ui);
        });

        let mut dock_state = std::mem::replace(&mut self.dock_state, DockState::new(vec![]));
        DockArea::new(&mut dock_state)
            .style(Style::from_egui(ctx.style().as_ref()))
            .show(ctx, &mut EditorTabViewer { app: self });
        self.dock_state = dock_state;
    }
}

fn main() -> eframe::Result<()> {
    let startup_path = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("DOT — Pixel & Icon Studio")
            .with_inner_size([1240.0, 780.0]),
        ..Default::default()
    };

    eframe::run_native(
        "DOT",
        options,
        Box::new(move |_cc| Ok(Box::new(IconEditorApp::from_startup_path(startup_path)))),
    )
}