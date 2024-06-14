//! like `egui::DragValue`, but imitating the blender version.

use egui::*;

pub struct DragValue<'a> {
    reference: &'a mut f32,

    range: Rangef,
    snap_fn: &'a dyn Fn(f32) -> f32,
    display: &'a dyn DragValueDisplay,
    name: Option<&'a str>,
}

impl<'a> DragValue<'a> {
    pub fn new(reference: &'a mut f32) -> Self {
        Self {
            reference,

            range: Rangef::new(0.0, 1.0),
            snap_fn: &std::convert::identity,
            display: &DefaultDragValueDisplay,
            name: None,
        }
    }
    pub fn range(self, range: Rangef) -> Self {
        Self { range, ..self }
    }
    pub fn snap_fn(self, snap_fn: &'a dyn Fn(f32) -> f32) -> Self {
        Self { snap_fn, ..self }
    }
    pub fn display(self, display: &'a dyn DragValueDisplay) -> Self {
        Self { display, ..self }
    }
    pub fn name(self, name: Option<&'a str>) -> Self {
        Self { name, ..self }
    }
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            reference,

            range,
            snap_fn,
            display,
            name,
        } = self;

        let shift = ui.input(|i| i.modifiers.shift_only());

        let id = ui.next_auto_id();

        let is_kb_editing = ui.memory_mut(|mem| {
            mem.interested_in_focus(id);
            mem.has_focus(id)
        });

        let mut value = *reference;

        let aim_rad = ui.input(|i| i.aim_radius());

        let change = ui.input_mut(|input| {
            let mut change = 0.0;

            if is_kb_editing {
                // see https://docs.rs/egui/latest/src/egui/widgets/drag_value.rs.html#407
                change += input.count_and_consume_key(Modifiers::NONE, Key::ArrowUp) as f32
                    - input.count_and_consume_key(Modifiers::NONE, Key::ArrowDown) as f32;
            }

            // TODO implement accesskit
            // #[cfg(feature = "accesskit")]
            // {
            //     use accesskit::Action;
            //     change += input.num_accesskit_action_requests(id, Action::Increment) as f32
            //         - input.num_accesskit_action_requests(id, Action::Decrement) as f32;
            // }

            change
        });

        if change != 0.0 {
            value = (self.snap_fn)(value + change);
        }

        let text_style = ui.style().drag_value_text_style.clone();

        let mut response = if is_kb_editing {
            let mut value_text = ui
                .data_mut(|data| data.remove_temp::<String>(id))
                .unwrap_or_else(|| self.display.to_input(value));
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .clip_text(false)
                    .horizontal_align(ui.layout().horizontal_align())
                    .vertical_align(ui.layout().vertical_align())
                    .margin(ui.spacing().button_padding)
                    .min_size(ui.spacing().interact_size)
                    .id(id)
                    .desired_width(f32::INFINITY)
                    .font(text_style),
            );

            if response.lost_focus() {
                if let Some(parsed_value) = self.display.from_input(value_text) {
                    *reference = parsed_value;
                }
            } else {
                ui.data_mut(|data| data.insert_temp(id, value_text));
            }

            response
        } else {
            let value_text = self.display.to_display(value);
            let num_chars = value_text.chars().count();

            let galley = WidgetText::from(value_text).into_galley(
                ui,
                Some(false),
                f32::INFINITY,
                TextStyle::Button,
            );

            let desired_height = galley.size().y;

            let (rect, response) = ui.allocate_at_least(
                vec2(ui.max_rect().width(), desired_height),
                Sense::click_and_drag(),
            );

            let mut response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);

            // if ui.style().explanation_tooltips {
            //     response = response.on_hover_text(format!(
            //         "{}{}{}\nDrag to edit or click to enter a value.\nPress 'Shift' while dragging for better control.",
            //         prefix,
            //         value as f32,
            //         suffix
            //     ));
            // }

            if ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released()) {
                ui.data_mut(|data| data.remove::<f32>(id));
            }

            if response.clicked() {
                ui.data_mut(|data| data.remove::<String>(id));
                ui.memory_mut(|mem| mem.request_focus(id));
                let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                state.cursor.set_char_range(Some(text::CCursorRange::two(
                    text::CCursor::default(),
                    text::CCursor::new(num_chars),
                )));
                state.store(ui.ctx(), response.id);
            } else if response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);

                let mdelta = response.drag_delta();
                let delta_points = mdelta.x - mdelta.y;

                let speed = self.range.span() / rect.width();
                let speed = if shift { speed / 10.0 } else { speed };

                let delta_value = delta_points * speed;

                if delta_value != 0.0 {
                    let precise_value = ui.data_mut(|data| data.get_temp::<f32>(id));
                    let precise_value = precise_value.unwrap_or(value);
                    let precise_value = precise_value + delta_value;

                    let aim_delta = aim_rad * speed;
                    let value = emath::smart_aim::best_in_range_f64(
                        (precise_value - aim_delta) as f64,
                        (precise_value + aim_delta) as f64,
                    ) as f32;
                    let rounded_value = (self.snap_fn)(value);
                    let clamped_value = self.range.clamp(rounded_value);

                    *reference = clamped_value;

                    ui.data_mut(|data| data.insert_temp::<f32>(id, precise_value));
                }
            }

            response
        };

        response
    }
}

pub trait DragValueDisplay {
    fn to_display(&self, val: f32) -> String;
    fn to_input(&self, val: f32) -> String {
        self.to_display(val)
    }
    // TODO implement expression evaluator based off of https://crates.io/crates/meval or the like
    fn from_input(&self, str: String) -> Option<f32>;
}

pub struct DefaultDragValueDisplay;
impl DragValueDisplay for DefaultDragValueDisplay {
    fn to_display(&self, val: f32) -> String {
        format!("{val:.2}")
    }
    fn from_input(&self, str: String) -> Option<f32> {
        str.parse().ok()
    }
}
