use cubedaw_lib::{Id, Note, Range, Section, Track};
use egui::{pos2, vec2, Color32, Pos2, Rangef, Rect, Rounding};
use smallvec::SmallVec;

use crate::app::Tab;

use super::track::TrackTab;

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track: Option<Id<Track>>,

    vertical_zoom: f32,
    horizontal_zoom: f32,

    last_mouse_position: (i64, i32),

    currently_drawn_note: Option<Note>,
}

const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT;
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

fn snap_pos(pos: i64, horizontal_zoom: f32) -> i64 {
    let step = ((Range::UNITS_PER_BEAT as f32 / horizontal_zoom * 0.05).min(256.0) as u32)
        .next_power_of_two() as i64;

    (pos + step / 2).div_floor(step) * step
}

impl crate::Screen for PianoRollTab {
    fn create(ctx: &mut crate::Context) -> Self {
        Self {
            id: Id::arbitrary(),

            track: ctx
                .tabs
                .get_tab::<TrackTab>()
                .map(|t| t.get_single_selected_track()),

            vertical_zoom: 16.0,
            horizontal_zoom: 0.5,

            last_mouse_position: (0, 0),

            currently_drawn_note: None,
        }
    }

    fn id(&self) -> Id<Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Piano Roll".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().show_viewport(ui, |ui, viewport| {
                if self.track.is_some() {
                    self.pianoroll(ctx, ui, viewport);
                } else {
                    self.pianoroll_empty(ui);
                }
            })
        });
    }
}

impl PianoRollTab {
    fn pianoroll(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, viewport: Rect) {
        let Some(track_id) = self.track else {
            unreachable!()
        };

        let mut single_thing_clicked: Option<Id> = None;

        let track = ctx.state.tracks.get_mut(track_id);

        let (_, canvas_resp) = ui.allocate_exact_size(
            vec2(
                (ctx.state.song_boundary.length() + SONG_PADDING * 2) as f32 * self.horizontal_zoom,
                (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * self.vertical_zoom,
            ),
            egui::Sense::click_and_drag(),
        );

        // Note area

        let max_rect = ui.max_rect();
        let top_left = max_rect.left_top().to_vec2();
        let painter = ui.painter_at(viewport.translate(top_left));

        let screen_rect = viewport.translate(top_left);

        painter.rect_filled(max_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);

        let screen_pos_to_note_pos = |screen_pos: Pos2| -> (i64, i32) {
            let ui_pos = screen_pos - top_left;
            (
                (ui_pos.x / self.horizontal_zoom) as i64 + ctx.state.song_boundary.start
                    - SONG_PADDING,
                (ui_pos.y / self.vertical_zoom) as i32 + MIN_NOTE_SHOWN,
            )
        };
        let note_pos_to_screen_pos = |(pos, pitch): (i64, i32)| -> Pos2 {
            let ui_pos = pos2(
                (pos - ctx.state.song_boundary.start + SONG_PADDING) as f32 * self.horizontal_zoom,
                (pitch - MIN_NOTE_SHOWN) as f32 * self.vertical_zoom,
            );
            ui_pos + top_left
        };
        let pos_range_to_screen_range = |range: Range| -> Rangef {
            Rangef::new(
                (range.start - ctx.state.song_boundary.start + SONG_PADDING) as f32
                    * self.horizontal_zoom
                    + top_left.x,
                (range.end - ctx.state.song_boundary.start + SONG_PADDING) as f32
                    * self.horizontal_zoom
                    + top_left.x,
            )
        };

        let (min_pos, min_pitch) = (
            (viewport.left() / self.horizontal_zoom) as i64 + ctx.state.song_boundary.start
                - SONG_PADDING,
            (viewport.top() / self.vertical_zoom) as i32 + MIN_NOTE_SHOWN,
        );

        let (max_pos, max_pitch) = (
            (viewport.right() / self.horizontal_zoom) as i64 + ctx.state.song_boundary.start
                - SONG_PADDING,
            (viewport.bottom() / self.vertical_zoom) as i32 + MIN_NOTE_SHOWN,
        );

        // The horizontal "note lines"
        for row in min_pitch..=max_pitch {
            if row % 2 == 0 {
                let row_pos = note_pos_to_screen_pos((0, row)).y;
                painter.rect_filled(
                    Rect::from_x_y_ranges(
                        screen_rect.x_range(),
                        Rangef::new(row_pos, row_pos + self.vertical_zoom),
                    ),
                    Rounding::ZERO,
                    ui.visuals().faint_bg_color.gamma_multiply(2.0),
                );
            }
        }

        // Vertical bar/beat/whatever indicators

        let vbar_step = ((Range::UNITS_PER_BEAT as f32 / self.horizontal_zoom * 0.1).min(256.0)
            as u32)
            .next_power_of_two() as i64;

        // TODO make this not hardcoded
        const BEATS_PER_BAR: i64 = 4;

        for i in min_pos.div_ceil(vbar_step)..=max_pos.div_floor(vbar_step) {
            let pos = i * vbar_step;
            let stroke = if pos % (BEATS_PER_BAR * Range::UNITS_PER_BEAT) == 0 {
                ui.visuals().widgets.hovered.bg_stroke
            } else {
                let division = pos
                    .trailing_zeros()
                    .min(Range::UNITS_PER_BEAT.trailing_zeros());
                egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.hovered.bg_stroke.color.gamma_multiply(
                        (division as f32 / Range::UNITS_PER_BEAT.trailing_zeros() as f32).powi(2),
                    ),
                )
            };
            painter.vline(
                painter.round_to_pixel(note_pos_to_screen_pos((pos, 0)).x),
                screen_rect.y_range(),
                stroke,
            );
        }

        // Sections

        // TODO implement section colors
        const SECTION_COLOR: Color32 = Color32::from_rgb(145, 0, 235);

        let draw_note = |offset, note: &Note| {
            let note_screen_range_x = pos_range_to_screen_range(note.range + offset);

            let note_y = note_pos_to_screen_pos((0, note.pitch)).y;

            painter.rect_filled(
                Rect::from_x_y_ranges(
                    note_screen_range_x,
                    Rangef::new(note_y, note_y + self.vertical_zoom),
                ),
                Rounding::ZERO,
                Color32::DEBUG_COLOR,
            );
        };

        // using a closure would result in a "reference cannot escape FnMut" or whatever so a macro is needed here
        macro_rules! prepare_section {
            ($section:tt) => {
                ({
                    let (section_range, section_id) = $section;

                    let section_ui_data = ctx.ui_state.sections.get_mut_or_default(section_id);

                    let section_range = if let Some(ref section_drag) = ctx.ui_state.section_drag {
                        if section_drag.0.contains(&section_id) {
                            section_range + section_drag.2
                        } else {
                            section_range
                        }
                    } else {
                        section_range
                    };

                    if section_range.start > max_pos {
                        continue;
                    } else if section_range.end < min_pos {
                        continue;
                    } else {
                        let section = ctx.state.sections.get_mut(section_id);

                        (section_id, section, section_ui_data, section_range)
                    }
                })
            };
        }

        for section in track.sections() {
            let (section_id, section, section_ui_data, section_range) = prepare_section!(section);

            let section_stroke = egui::Stroke::new(
                2.0,
                SECTION_COLOR
                    .gamma_multiply(0.5 * if section_ui_data.selected { 1.5 } else { 1.0 }),
            );

            let section_screen_range_x = pos_range_to_screen_range(section_range);

            painter.rect_filled(
                Rect::from_x_y_ranges(section_screen_range_x, screen_rect.y_range()),
                Rounding::ZERO,
                SECTION_COLOR
                    .gamma_multiply(0.2 * if section_ui_data.selected { 1.5 } else { 1.0 }),
            );
            painter.vline(
                section_screen_range_x.min,
                screen_rect.y_range(),
                section_stroke,
            );
            painter.vline(
                section_screen_range_x.max,
                screen_rect.y_range(),
                section_stroke,
            );

            // Notes
            for note in section.notes() {
                draw_note(section_range.start, note);
            }
        }
        if let Some(ref note) = self.currently_drawn_note {
            draw_note(0, note);
        }

        // Top area (for displaying sections, etc)

        const TOP_BAR_HEIGHT: f32 = 18.0;
        let top_bar_rect = Rect::from_x_y_ranges(
            screen_rect.x_range(),
            Rangef::new(screen_rect.top(), screen_rect.top() + TOP_BAR_HEIGHT),
        );

        painter.rect_filled(top_bar_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);
        painter.hline(
            top_bar_rect.x_range(),
            top_bar_rect.bottom(),
            ui.visuals().window_stroke,
        );

        // Section headers
        let mut finished_drag_offset = None;
        let mut drag_started = false;
        for section in track.sections() {
            let (section_id, section, section_ui_data, section_range) = prepare_section!(section);

            let section_screen_range_x = pos_range_to_screen_range(section_range);

            let header_rect =
                Rect::from_x_y_ranges(section_screen_range_x.expand(1.0), top_bar_rect.y_range());

            let header_resp = ui.allocate_rect(header_rect, egui::Sense::click_and_drag());

            painter.rect_filled(
                header_rect,
                Rounding {
                    nw: 6.0,
                    ne: 6.0,
                    sw: 0.0,
                    se: 0.0,
                },
                if section_ui_data.selected {
                    SECTION_COLOR.gamma_multiply(0.7)
                } else {
                    SECTION_COLOR.gamma_multiply(0.5)
                },
            );

            let padding = header_rect.height() * 0.5;
            painter.text(
                pos2(header_rect.left() + padding, header_rect.top() + padding),
                egui::Align2::LEFT_CENTER,
                &section.name,
                egui::FontId::proportional(12.0),
                if section_ui_data.selected {
                    &ui.visuals().widgets.hovered
                } else {
                    ui.visuals().widgets.style(&header_resp)
                }
                .text_color(),
            );

            if header_resp.clicked() || header_resp.drag_started() {
                if header_resp.drag_started() {
                    drag_started = true;
                    ctx.ui_state.section_drag = Some((Default::default(), 0.0, 0));
                }
                if header_resp.clicked() || !section_ui_data.selected {
                    section_ui_data.selected = true;
                    if !ui.input(|i| i.modifiers.shift) {
                        single_thing_clicked = Some(section_id.transmute());
                    }
                }
            }
            if header_resp.dragged() {
                let drag_offset = ctx
                    .ui_state
                    .section_drag
                    .as_ref()
                    .map(|x| x.1)
                    .unwrap_or_default()
                    + header_resp.drag_delta().x / self.horizontal_zoom;
                let snapped_drag_offset = drag_offset as i64;
                let snapped_drag_offset =
                    snap_pos(section.start() + snapped_drag_offset, self.horizontal_zoom)
                        - section.start();
                let section_drag = ctx.ui_state.section_drag.as_mut().unwrap();
                section_drag.1 = drag_offset;
                section_drag.2 = snapped_drag_offset;
            }
            if header_resp.drag_released() {
                if let Some(ref drag_offset) = ctx.ui_state.section_drag {
                    finished_drag_offset = Some(drag_offset.2);
                } else {
                    unreachable!();
                }
            }
        }

        if drag_started {
            let section_drag = ctx.ui_state.section_drag.as_mut().unwrap();
            for (_, section_id) in track.sections() {
                if ctx.ui_state.sections.get(section_id).selected {
                    section_drag.0.insert(section_id);
                }
            }
        }
        if let Some(finished_drag_offset) = finished_drag_offset {
            ctx.ui_state.section_drag = None;

            let mut sections_to_move = SmallVec::<[Range; 8]>::new();
            for (section_range, section_id) in track.sections() {
                let section_ui_data = ctx.ui_state.sections.get_mut_or_default(section_id);
                if section_ui_data.selected {
                    sections_to_move.push(section_range);
                }
            }
            for &section_range in &sections_to_move {
                track.move_section(
                    &mut ctx.state.sections,
                    section_range,
                    section_range + finished_drag_offset,
                );
            }
            track.check_overlap();
        }

        // Bar indicators
        for bar in min_pos.div_floor(BEATS_PER_BAR * Range::UNITS_PER_BEAT)
            ..=max_pos.div_ceil(BEATS_PER_BAR * Range::UNITS_PER_BEAT)
        {
            let pos = bar * (BEATS_PER_BAR * Range::UNITS_PER_BEAT);
            painter.text(
                pos2(note_pos_to_screen_pos((pos, 0)).x, top_bar_rect.center().y),
                egui::Align2::CENTER_CENTER,
                bar.to_string(),
                egui::FontId::proportional(12.0),
                ui.visuals().widgets.hovered.text_color(),
            );
        }

        let track = ctx.state.tracks.get_mut(track_id);

        if let Some(mouse_pos) = canvas_resp.hover_pos() {
            let (pos, pitch) = screen_pos_to_note_pos(mouse_pos);
            let pos = snap_pos(pos, self.horizontal_zoom);
            self.last_mouse_position = (pos, pitch);

            if ui.input(|i| i.modifiers.ctrl) {
                let mouse_down = ui.input(|i| i.pointer.primary_down());

                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                if mouse_down {
                    if let Some(ref mut currently_drawn_note) = self.currently_drawn_note {
                        currently_drawn_note.range.end = currently_drawn_note.start().max(pos);
                    } else {
                        self.currently_drawn_note =
                            Some(Note::from_range_pitch(Range::at(pos), pitch));
                    }
                } else {
                    if let Some(note) = self.currently_drawn_note.take() {
                        let section = match track.get_section_at(note.start()) {
                            Some(id) => ctx.state.sections.get_mut(id),
                            None => {
                                let section = Section::empty(
                                    "New Section".into(),
                                    Range::surrounding_pos(note.start()),
                                );
                                track.check_overlap_with(section.range);
                                track.add_section(&mut ctx.state.sections, section)
                            }
                        };
                        section.insert_note(note);
                    }

                    let screen_pos = note_pos_to_screen_pos((pos, pitch));
                    painter.vline(
                        screen_pos.x,
                        Rangef::new(screen_pos.y, screen_pos.y + self.vertical_zoom),
                        if mouse_down {
                            ui.visuals().widgets.active
                        } else {
                            ui.visuals().widgets.hovered
                        }
                        .fg_stroke,
                    );
                }
            }
        }

        canvas_resp.context_menu(|ui| {
            if ui.button("Add section").clicked() {
                track.add_section(
                    &mut ctx.state.sections,
                    Section::empty(
                        "New Section".into(),
                        Range::surrounding_pos(self.last_mouse_position.0),
                    ),
                );
                ui.close_menu();
            }
        });

        // self.ui_data.delete_unneeded(&mut ctx.state.sections);

        if canvas_resp.clicked() && single_thing_clicked.is_none() {
            single_thing_clicked = Some(Id::invalid());
        }

        if let Some(single_thing_clicked) = single_thing_clicked {
            // deselect all the other sections
            for (&id, ui_data) in ctx.ui_state.sections.iter_mut() {
                if id.transmute() != single_thing_clicked {
                    ui_data.selected = false;
                }
            }
        }
    }

    fn pianoroll_empty(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                ui.label("No track selected");
            },
        );
    }
}
