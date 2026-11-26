pub(crate) mod eraser;
pub(crate) mod eyedropper;
pub(crate) mod fill;
pub(crate) mod pencil;
pub(crate) mod select_rect;

pub(crate) use eraser::EraserProps;
pub(crate) use eyedropper::EyedropperProps;
pub(crate) use fill::FillProps;
pub(crate) use pencil::PencilProps;
pub(crate) use select_rect::SelectRectProps;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BrushShape {
    Square,
    Circle,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BrushParams {
    pub(crate) size: u32,
    pub(crate) shape: BrushShape,
    pub(crate) mirror_x: bool,
    pub(crate) mirror_y: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ToolSettings {
    pub(crate) pencil: PencilProps,
    pub(crate) eraser: EraserProps,
    pub(crate) fill: FillProps,
    pub(crate) eyedropper: EyedropperProps,
    pub(crate) select_rect: SelectRectProps,
}