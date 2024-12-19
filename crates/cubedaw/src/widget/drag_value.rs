//! Like `egui::DragValue`, but imitating the Blender version. Because it looks cool.

use egui::{emath::inverse_lerp, text_edit::TextEditState, *};

use super::InputModifiers;

pub type ExtraFunc<'a> = Box<dyn FnOnce(&mut egui::Ui) + 'a>;

/// A Blender-like draggable slider.
pub struct DragValue<'a> {
    reference: &'a mut f32,

    show_number_text: bool,
    interactable: bool,
    relative: bool,
    range: Rangef,
    display_range: Rangef,
    display: &'a dyn ValueHandler,
    name: Option<&'a str>,

    extra: Option<ExtraFunc<'a>>,
}

pub struct DefaultValueDisplay;
impl ValueHandler for DefaultValueDisplay {
    fn to_input(&self, val: f32, ctx: &ValueHandlerContext) -> String {
        if ctx.is_relative {
            format!("{val:+.2}")
        } else {
            format!("{val:.2}")
        }
    }
    fn parse_input(&self, str: &str, _ctx: &ValueHandlerContext) -> Option<f32> {
        str.parse().ok()
    }
    fn snap(&self, val: f32, ctx: &ValueHandlerContext) -> f32 {
        if ctx.alternate() {
            // don't round
            val
        } else {
            (val * 100.0).round() * 0.01
        }
    }
}

impl<'a> DragValue<'a> {
    pub fn new(reference: &'a mut f32) -> Self {
        Self {
            reference,

            show_number_text: true,
            interactable: true,
            relative: false,
            range: Rangef::EVERYTHING,
            display_range: Rangef::new(0.0, 1.0),
            display: &DefaultValueDisplay,
            name: None,

            extra: None,
        }
    }
    pub fn show_number_text(self, show_number_text: bool) -> Self {
        Self {
            show_number_text,
            ..self
        }
    }
    pub fn interactable(self, interactable: bool) -> Self {
        Self {
            interactable,
            ..self
        }
    }
    pub fn relative(self, relative: bool) -> Self {
        Self { relative, ..self }
    }
    pub fn range(self, range: Rangef) -> Self {
        Self { range, ..self }
    }
    pub fn display_range(self, display_range: Rangef) -> Self {
        Self {
            display_range,
            ..self
        }
    }
    pub fn display(self, display: &'a dyn ValueHandler) -> Self {
        Self { display, ..self }
    }
    pub fn name(self, name: Option<&'a str>) -> Self {
        Self { name, ..self }
    }

    pub fn extra(self, extra: Option<ExtraFunc<'a>>) -> Self {
        Self { extra, ..self }
    }
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            reference,

            show_number_text,
            interactable,
            relative,
            range,
            display_range,
            display,
            name,

            extra,
        } = self;

        let response = ui.scope_builder(egui::UiBuilder::new().layout(egui::Layout::right_to_left(egui::Align::Min)), |ui| {
            ui.spacing_mut().combo_width = 0.0;
            if let Some(extra) = extra {
                extra(ui);
            }

            let ctx = ValueHandlerContext {
                is_relative: relative,
                modifiers: ui.input(InputModifiers::read_from_egui_input),
            };

            let mut padding = ui.spacing().button_padding;
            // TODO: we're 2 units off height-wise from a button/combobox/etc and i don't know why
            padding.y += 1.0;


            let id = ui.next_auto_id();

            let is_kb_editing = ui.memory_mut(|mem| {
                mem.interested_in_focus(id);
                mem.has_focus(id)
            });

            let mut value = *reference;

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
                value = display.snap(value + change, &ctx);
            }

            let text_style = ui.style().drag_value_text_style.clone();

            let response = if is_kb_editing {
                let mut value_text = ui
                    .data_mut(|data| data.remove_temp::<String>(id))
                    .unwrap_or_else(|| {
                        // this shouldn't ever happen (we set the data when transitioning to kb editing)
                        // but it's not a catastrophic issue if it does. default value go brrrrr
                        display.to_input(value, &ctx)
                    });
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
                    if let Some(parsed_value) = display.parse_input(value_text.trim(), &ctx) {
                        *reference = parsed_value;
                    }
                } else {
                    ui.data_mut(|data| data.insert_temp(id, value_text));
                }

                response
            } else {
                let width = ui.available_size_before_wrap().x;

                let number_galley = show_number_text.then(|| {
                    let value_text = display.to_display(value, &ctx);
                    value_text.into_galley(ui, None, width, TextStyle::Button)
                });
                let name_galley = name.map(|name| {
                    WidgetText::from(name).into_galley(
                        ui,
                        None,
                        width
                            - number_galley
                                .as_ref()
                                .map_or(0.0, |galley| galley.rect.width()),
                        TextStyle::Button,
                    )
                });

                let mut text_height = number_galley.as_ref().map_or(8.0, |galley| galley.size().y);
                if let Some(ref name_galley) = name_galley {
                    let name_height = name_galley.size().y;
                    if name_height > text_height {
                        text_height = name_height;
                    }
                }

                let desired_height = text_height + padding.y * 2.0;

                let (rect, mut response) = ui.allocate_at_least(
                    vec2(width, desired_height),
                    if interactable {
                        Sense::click_and_drag()
                    } else {
                        Sense::hover()
                    },
                );
                if interactable {
                    response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);
                }

                // TODO make configurable
                if response.hovered()
                    && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
                {
                    *reference = display.default_value(&ctx);
                }

                if ui.style().explanation_tooltips {
                    response = response.on_hover_text(format!(
                        "{}\nDrag to edit or click to enter a value.\nPress 'Shift' while dragging for better control.",
                        value,
                    ));
                }

                if ui.input(|i| i.pointer.any_pressed() || i.pointer.any_released()) {
                    ui.data_mut(|data| data.remove::<f32>(id));
                }

                if response.clicked() {
                    ui.memory_mut(|mem| mem.request_focus(id));

                    let input_text = display.to_input(value, &ctx);
                    let mut state = TextEditState::default();
                    state.cursor.set_char_range(Some(text::CCursorRange::two(
                        text::CCursor::default(),
                        text::CCursor::new(input_text.chars().count()),
                    )));
                    state.store(ui.ctx(), response.id);
                    ui.data_mut(|data| data.insert_temp::<String>(id, input_text));
                } else if response.dragged() {
                    // TODO egui can't lock the cursor like blender does. is that behavior necessary?
                    // if it is, how do we do that
                    // ui.ctx().set_cursor_icon(CursorIcon::None);

                    if response.drag_started()
                        && range != Rangef::EVERYTHING
                        && let Some(drag_pos) = response.interact_pointer_pos()
                    {
                        if let Some(initial_value) = inverse_lerp(rect.x_range().into(), drag_pos.x) {
                            value = display.snap(lerp(display_range, initial_value), &ctx);
                            *reference = value;
                        }
                    }

                    let delta_points = response.drag_delta().x;

                    let speed = display_range.span() / rect.width();
                    let speed = if ctx.alternate() { speed * 0.1 } else { speed };

                    let delta_value = delta_points * speed;

                    if delta_value != 0.0 {
                        let precise_value = ui.data(|data| data.get_temp::<f32>(id));
                        let precise_value = precise_value.unwrap_or(value);
                        let precise_value = precise_value + delta_value;

                        let mut value = range.clamp(precise_value);
                        value = display.snap(value, &ctx);
                        *reference = value;

                        ui.data_mut(|data| data.insert_temp::<f32>(id, precise_value));
                    }
                }

                if ui.is_rect_visible(rect) {
                    let visuals = ui.style().interact(&response);

                    let painter = ui.painter();

                    let rect_without_padding = rect.shrink2(padding);

                    // non-text display
                    {
                        painter.rect(rect.expand(visuals.expansion), visuals.rounding, visuals.bg_fill, visuals.bg_stroke);

                        let portion_filled =
                            inverse_lerp(display_range.into(), *reference).unwrap_or(display_range.min);
                        painter.rect_filled(
                            rect.expand(visuals.expansion - visuals.bg_stroke.width * 0.5)
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

                        if let Some(number_galley) = number_galley {
                            let text_pos = text_layout
                                .align_size_within_rect(number_galley.size(), rect_without_padding)
                                .min;

                            painter.galley(text_pos, number_galley, visuals.text_color());
                        }
                    }
                }

                response
            };
            response
        }).inner;

        response
    }
}

pub trait ValueHandler {
    fn to_display(&self, val: f32, ctx: &ValueHandlerContext) -> WidgetText {
        self.to_input(val, ctx).into()
    }
    fn to_input(&self, val: f32, ctx: &ValueHandlerContext) -> String;
    // TODO implement expression evaluator based off of https://crates.io/crates/meval or the like
    fn parse_input(&self, str: &str, ctx: &ValueHandlerContext) -> Option<f32>;

    fn snap(&self, val: f32, ctx: &ValueHandlerContext) -> f32;

    fn default_value(&self, _ctx: &ValueHandlerContext) -> f32 {
        0.0
    }
}

pub struct ValueHandlerContext {
    pub is_relative: bool,
    pub modifiers: InputModifiers,
}
impl ValueHandlerContext {
    pub fn alternate(&self) -> bool {
        self.modifiers.contains(InputModifiers::ALTERNATE)
    }
}
