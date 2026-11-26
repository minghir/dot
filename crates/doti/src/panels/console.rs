use crate::IconEditorApp;
use eframe::egui;

impl IconEditorApp {
    pub(crate) fn run_dot_command(&mut self, raw: &str) {
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

        let base_dir = std::path::Path::new(self.dot_script_path.trim())
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(std::path::Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok());

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
                base_dir,
                imported_files: std::collections::HashSet::new(),
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

    pub(crate) fn load_dot_script_from_path(&mut self, auto_run: bool) {
        let trimmed = self.dot_script_path.trim().trim_matches('"');
        if trimmed.is_empty() {
            self.cmd_history
                .push("Eroare dot: introdu calea către un fișier .dot.".to_string());
            return;
        }

        match std::fs::read_to_string(trimmed) {
            Ok(contents) => {
                self.cmd_input = contents.clone();
                self.cmd_history.push(format!("[încărcat: {trimmed}]"));
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

    pub(crate) fn show_console_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("(Ctrl+Enter sau Run)");
            if ui.small_button("Clear").clicked() {
                self.cmd_history.clear();
            }
            if ui.small_button("Reset Env").clicked() {
                self.cmd_vars.clear();
                self.cmd_funcs.clear();
                self.cmd_structs.clear();
                self.cmd_history
                    .push("[variabile, funcții și struct-uri resetate]".to_string());
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

            let enter_on_path =
                path_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.small_button("Load & Run").clicked() || enter_on_path {
                self.load_dot_script_from_path(true);
            }
        });

        ui.separator();

        let history_height = (ui.available_height() - 78.0_f32).max(40.0_f32);
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(history_height)
            .show(ui, |ui| {
                for entry in &self.cmd_history {
                    ui.monospace(entry);
                }
            });

        let open_braces = self.cmd_input.chars().fold(0_i32, |acc, c| match c {
            '{' => acc + 1,
            '}' => acc - 1,
            _ => acc,
        });

        let ctrl_enter = ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter));
        let plain_enter_single_line = !self.cmd_input.contains('\n')
            && open_braces <= 0
            && ui.input(|i| {
                !i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Enter)
            });

        ui.horizontal_top(|ui| {
            ui.monospace(if open_braces > 0 { ".." } else { ">>" });
            let edit = ui.add(
                egui::TextEdit::multiline(&mut self.cmd_input)
                    .desired_rows(3)
                    .desired_width((ui.available_width() - 72.0_f32).max(120.0_f32))
                    .code_editor()
                    .hint_text("Scrie cod dot-lang (import, struct, func, circle, rect...)"),
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
    }
}