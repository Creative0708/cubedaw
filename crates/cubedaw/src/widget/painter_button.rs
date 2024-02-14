use egui::{
    style::WidgetVisuals, Color32, Painter, Response, Rounding, Sense, Stroke, Vec2, Widget,
    WidgetInfo, WidgetType,
};

pub trait PainterButtonCallback: FnOnce(Painter, bool, &WidgetVisuals) -> () {}
impl<T: FnOnce(Painter, bool, &WidgetVisuals) -> ()> PainterButtonCallback for T {}

/// A button with a custom painter (for icons in buttons.)
///
/// This is basically a clone of [`egui::Button`].
pub struct PainterButton<F: PainterButtonCallback> {
    paint: F,

    size: Vec2,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    sense: Sense,
    frame: Option<bool>,
    rounding: Option<Rounding>,

    selected: bool,
}

impl<F: PainterButtonCallback> PainterButton<F> {
    pub fn new(paint: F) -> PainterButton<F> {
        Self {
            paint,

            size: Vec2::splat(24.0),
            fill: None,
            stroke: None,
            sense: Sense::click(),
            frame: None,
            rounding: None,

            selected: false,
        }
    }
    pub fn size(mut self, size: Vec2) -> Self {
        debug_assert!(size.x >= 0.0 && size.y >= 0.0);
        self.size = size;
        self
    }

    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self.frame = Some(true);
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self.frame = Some(true);
        self
    }

    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = Some(rounding.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl<F: PainterButtonCallback> Widget for PainterButton<F> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let PainterButton {
            paint,
            size,
            selected,
            fill,
            stroke,
            sense,
            frame,
            rounding,
        } = self;
        let (rect, response) = ui.allocate_at_least(size, sense);
        response.widget_info(|| WidgetInfo::new(WidgetType::Button));

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            let (frame_expansion, frame_rounding, frame_fill, frame_stroke) = if selected {
                let selection = ui.visuals().selection;
                (0.0, Rounding::ZERO, selection.bg_fill, selection.stroke)
            } else if frame.unwrap_or_else(|| ui.visuals().button_frame) {
                (
                    visuals.expansion,
                    visuals.rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                )
            } else {
                Default::default()
            };
            ui.painter().rect(
                rect.expand(frame_expansion),
                rounding.unwrap_or(frame_rounding),
                fill.unwrap_or(frame_fill),
                stroke.unwrap_or(frame_stroke),
            );

            paint(ui.painter_at(rect), selected, visuals);
        }

        if response.hovered() {
            if let Some(cursor) = ui.visuals().interact_cursor {
                ui.ctx().set_cursor_icon(cursor);
            }
        }

        response
    }
}
