//! like `egui::DragValue`, but imitating the blender version.

use egui::{emath::inverse_lerp, *};

/// A Blender-like draggable slider.
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

        let padding = ui.spacing().button_padding;

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
            value = snap_fn(value + change);
        }

        let text_style = ui.style().drag_value_text_style.clone();

        let mut response = if is_kb_editing {
            let mut value_text = ui
                .data_mut(|data| data.remove_temp::<String>(id))
                .unwrap_or_else(|| display.to_input(value));
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .clip_text(false)
                    .horizontal_align(ui.layout().horizontal_align())
                    .vertical_align(ui.layout().vertical_align())
                    .margin(padding)
                    .min_size(ui.spacing().interact_size)
                    .id(id)
                    .desired_width(f32::INFINITY)
                    .font(text_style),
            );

            if response.lost_focus() {
                if let Some(parsed_value) = display.parse_input(value_text) {
                    *reference = parsed_value;
                }
            } else {
                ui.data_mut(|data| data.insert_temp(id, value_text));
            }

            response
        } else {
            let value_text = display.to_display(value);
            let num_chars = value_text.chars().count();

            let max_width = ui.max_rect().width();

            let number_galley = WidgetText::from(value_text).into_galley(
                ui,
                Some(false),
                max_width,
                TextStyle::Button,
            );
            let name_galley = name.map(|name| {
                WidgetText::from(name).into_galley(
                    ui,
                    Some(false),
                    max_width - number_galley.rect.width(),
                    TextStyle::Button,
                )
            });

            let mut text_height = number_galley.size().y;
            if let Some(ref name_galley) = name_galley {
                let name_height = name_galley.size().y;
                if name_height > text_height {
                    text_height = name_height;
                }
            }

            let desired_height = text_height + padding.y * 2.0;

            let (rect, response) =
                ui.allocate_at_least(vec2(max_width, desired_height), Sense::click_and_drag());

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
                ui.ctx().set_cursor_icon(CursorIcon::None);

                if response.drag_started()
                    && let Some(drag_pos) = response.interact_pointer_pos()
                {
                    if let Some(initial_value) = inverse_lerp(rect.x_range().into(), drag_pos.x) {
                        value = snap_fn(lerp(range, initial_value));
                        *reference = value;
                    }
                }

                let delta_points = response.drag_delta().x;

                let mut speed = range.span() / rect.width();
                let layer_id = ui.layer_id();
                if let Some(scale) = ui.ctx().memory(|m| {
                    m.layer_transforms
                        .get(&layer_id)
                        .map(|transform| transform.scaling)
                }) {
                    speed *= scale;
                }
                let speed = if shift { speed * 0.1 } else { speed };

                let delta_value = delta_points * speed;

                if delta_value != 0.0 {
                    let precise_value = ui.data(|data| data.get_temp::<f32>(id));
                    let precise_value = precise_value.unwrap_or(value);
                    let precise_value = precise_value + delta_value;

                    let aim_delta = aim_rad * speed;
                    let value = emath::smart_aim::best_in_range_f64(
                        (precise_value - aim_delta) as f64,
                        (precise_value + aim_delta) as f64,
                    ) as f32;

                    *reference = snap_fn(range.clamp(value));

                    ui.data_mut(|data| data.insert_temp::<f32>(id, precise_value));
                }
            }

            let visuals = ui.style().interact(&response);

            let painter = ui.painter();

            let rect_without_padding = rect.shrink2(padding);

            // non-text display
            {
                painter.rect(rect, visuals.rounding, visuals.bg_fill, visuals.bg_stroke);

                let portion_filled = inverse_lerp(range.into(), *reference).unwrap_or(range.min);
                painter.rect_filled(
                    rect.shrink(visuals.bg_stroke.width * 0.5)
                        .intersect(Rect::everything_left_of(lerp(
                            rect.x_range(),
                            portion_filled,
                        ))),
                    visuals.rounding,
                    ui.visuals().selection.bg_fill,
                );
            }

            // name text
            if let Some(name_galley) = name_galley {
                let text_pos = egui::Layout {
                    main_dir: egui::Direction::LeftToRight,
                    main_align: Align::Min,
                    ..Default::default()
                }
                .align_size_within_rect(name_galley.size(), rect_without_padding)
                .min;

                painter.galley(text_pos, name_galley, visuals.text_color());
            }

            // number text
            {
                let text_layout = if name.is_some() {
                    egui::Layout {
                        main_dir: egui::Direction::RightToLeft,
                        main_align: Align::Max,
                        ..Default::default()
                    }
                } else {
                    egui::Layout {
                        main_dir: egui::Direction::LeftToRight,
                        main_align: Align::Min,
                        ..Default::default()
                    }
                };

                let text_pos = text_layout
                    .align_size_within_rect(number_galley.size(), rect_without_padding)
                    .min;

                painter.galley(text_pos, number_galley, visuals.text_color());
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
    fn parse_input(&self, str: String) -> Option<f32>;
}

pub struct DefaultDragValueDisplay;
impl DragValueDisplay for DefaultDragValueDisplay {
    fn to_display(&self, val: f32) -> String {
        format!("{val:.2}")
    }
    fn parse_input(&self, str: String) -> Option<f32> {
        str.parse().ok()
    }
}
