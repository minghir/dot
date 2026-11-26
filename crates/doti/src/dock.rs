use crate::IconEditorApp;
use eframe::egui;
use egui_dock::{DockState, NodeIndex, TabViewer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PanelTab {
    Canvas,
    Tools,
    ToolProperties,
    Color,
    Frames,
    Layers,
    History,
    Console,
}

impl PanelTab {
    pub(crate) const ALL: &'static [PanelTab] = &[
        PanelTab::Canvas,
        PanelTab::Tools,
        PanelTab::ToolProperties,
        PanelTab::Color,
        PanelTab::Frames,
        PanelTab::Layers,
        PanelTab::History,
        PanelTab::Console,
    ];

    pub(crate) fn title(self) -> &'static str {
        match self {
            PanelTab::Canvas => "Canvas",
            PanelTab::Tools => "Unelte",
            PanelTab::ToolProperties => "Tool Properties",
            PanelTab::Color => "Culoare",
            PanelTab::Frames => "Formate .ICO",
            PanelTab::Layers => "Layers",
            PanelTab::History => "History",
            PanelTab::Console => "Console (dot)",
        }
    }

    pub(crate) fn is_closeable(self) -> bool {
        !matches!(self, PanelTab::Canvas)
    }
}

pub(crate) fn create_default_dock_state() -> DockState<PanelTab> {
    let mut dock_state = DockState::new(vec![PanelTab::Canvas]);

    // Stânga: Unelte sus, Tool Properties + Culoare jos
    let [center, left] = dock_state.main_surface_mut().split_left(
        NodeIndex::root(),
        0.20,
        vec![PanelTab::Tools],
    );
    let [left_top, left_bottom] = dock_state.main_surface_mut().split_below(
        left,
        0.34,
        vec![PanelTab::ToolProperties],
    );
    let _ = left_top;
    dock_state.main_surface_mut().split_below(
        left_bottom,
        0.42,
        vec![PanelTab::Color],
    );

    // Dreapta: Formate .ICO sus, Layers + History grupate ca tab-uri jos
    let [center, right] = dock_state.main_surface_mut().split_right(
        center,
        0.77,
        vec![PanelTab::Frames],
    );
    dock_state.main_surface_mut().split_below(
        right,
        0.52,
        vec![PanelTab::Layers, PanelTab::History],
    );

    // Jos sub Canvas: Consola dot-lang
    dock_state.main_surface_mut().split_below(
        center,
        0.74,
        vec![PanelTab::Console],
    );

    dock_state
}

pub(crate) struct EditorTabViewer<'a> {
    pub(crate) app: &'a mut IconEditorApp,
}

impl<'a> TabViewer for EditorTabViewer<'a> {
    type Tab = PanelTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        tab.is_closeable()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            PanelTab::Canvas => self.app.show_canvas_panel(ui),
            PanelTab::Tools => self.app.show_tools_panel(ui),
            PanelTab::ToolProperties => self.app.show_tool_properties_panel(ui),
            PanelTab::Color => self.app.show_color_panel(ui),
            PanelTab::Frames => self.app.show_frames_panel(ui),
            PanelTab::Layers => self.app.show_layers_panel(ui),
            PanelTab::History => self.app.show_history_panel(ui),
            PanelTab::Console => self.app.show_console_panel(ui),
        }
    }
}