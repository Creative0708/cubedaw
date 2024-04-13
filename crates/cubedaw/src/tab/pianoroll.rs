use cubedaw_lib::{Id, Note, Range, Section, Track};
use egui::{pos2, vec2, Color32, Pos2, Rangef, Rect, Rounding};
use smallvec::SmallVec;

use crate::app::Tab;

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track: Option<Id<Track>>,

    // Vertical zoom. Each note is this tall
    units_per_pitch: f32,
    // Horizontal zoom. Each tick is this wide
    units_per_tick: f32,

    last_mouse_position: (i64, i32),

    currently_drawn_note: Option<Note>,
}

// Number of empty ticks to display on either side of the song
const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT;
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

fn snap_pos(pos: i64, units_per_tick: f32) -> i64 {
    let step = ((Range::UNITS_PER_BEAT as f32 / units_per_tick * 0.05).min(256.0) as u32)
        .next_power_of_two() as i64;

    (pos + step / 2).div_floor(step) * step
}

impl crate::Screen for PianoRollTab {
    fn create(ctx: &mut crate::Context) -> Self {
        Self {
            id: Id::arbitrary(),

            track: {
                let mut selected_track = None;
                for &track_id in &ctx.ui_state.track_list {
                    let selected = ctx.ui_state.tracks.get(track_id).selected;
                    if selected {
                        if selected_track.is_some() {
                            selected_track = None;
                            break;
                        } else {
                            selected_track = Some(track_id);
                        }
                    }
                }
                selected_track
            },

            units_per_pitch: 16.0,
            units_per_tick: 0.5,

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

        let track = ctx.state.tracks.get_mut(track_id);

        let mut single_thing_clicked: Option<Id> = None;

        let (_, interaction) = ui.allocate_exact_size(
            vec2(
                (ctx.state.song_boundary.length() + SONG_PADDING * 2) as f32 * self.units_per_tick,
                (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * self.units_per_pitch,
            ),
            egui::Sense::click_and_drag(),
        );

        let max_rect = ui.max_rect();
        let top_left = max_rect.left_top().to_vec2();
        let painter = ui.painter_at(viewport.translate(top_left));

        let screen_rect = viewport.translate(top_left);

        painter.rect_filled(max_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);

        let screen_x_to_note_x = |pos: f32| -> i64 {
            ((pos - top_left.x) / self.units_per_tick) as i64 + ctx.state.song_boundary.start
                - SONG_PADDING
        };
        let screen_pos_to_note_pos = |screen_pos: Pos2| -> (i64, i32) {
            (
                screen_x_to_note_x(screen_pos.x),
                ((screen_pos.y - top_left.y) / self.units_per_pitch) as i32 + MIN_NOTE_SHOWN,
            )
        };
        let note_x_to_screen_x = |pos: f32| -> f32 {
            (pos - (ctx.state.song_boundary.start - SONG_PADDING) as f32) * self.units_per_tick
                + top_left.x
        };
        let note_pos_to_screen_pos = |(pos, pitch): (i64, i32)| -> Pos2 {
            let ui_pos = pos2(
                note_x_to_screen_x(pos as _),
                (pitch - MIN_NOTE_SHOWN) as f32 * self.units_per_pitch,
            );
            ui_pos + top_left
        };
        let pos_range_to_screen_range = |range: Range| -> Rangef {
            Rangef::new(
                note_x_to_screen_x(range.start as _),
                note_x_to_screen_x(range.end as _),
            )
        };

        let (min_pos, min_pitch) = (
            (viewport.left() / self.units_per_tick) as i64 + ctx.state.song_boundary.start
                - SONG_PADDING,
            (viewport.top() / self.units_per_pitch) as i32 + MIN_NOTE_SHOWN,
        );

        let (max_pos, max_pitch) = (
            (viewport.right() / self.units_per_tick) as i64 + ctx.state.song_boundary.start
                - SONG_PADDING,
            (viewport.bottom() / self.units_per_pitch) as i32 + MIN_NOTE_SHOWN,
        );

        // The horizontal "note lines"
        for row in min_pitch..=max_pitch {
            if row % 2 == 0 {
                let row_pos = note_pos_to_screen_pos((0, row)).y;
                painter.rect_filled(
                    Rect::from_x_y_ranges(
                        screen_rect.x_range(),
                        Rangef::new(row_pos, row_pos + self.units_per_pitch),
                    ),
                    Rounding::ZERO,
                    ui.visuals().faint_bg_color.gamma_multiply(2.0),
                );
            }
        }

        // Vertical bar/beat/whatever indicators

        let vbar_step = ((Range::UNITS_PER_BEAT as f32 / self.units_per_tick * 0.1).min(256.0)
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
                painter.round_to_pixel(note_x_to_screen_x(pos as _)),
                screen_rect.y_range(),
                stroke,
            );
        }

        // Sections

        // TODO implement section colors
        const SECTION_COLOR: Color32 = Color32::from_rgb(145, 0, 235);

        macro_rules! get_section_render_data {
            ($section:tt, $movement:tt, $sections:tt) => {{
                let (section_range, section_id) = $section;
                let section_ui_state = $sections.get_mut_or_default(section_id);

                let section_range = if let Some(section_drag) = $movement {
                    if section_ui_state.selected {
                        section_range + section_drag as i64
                    } else {
                        section_range
                    }
                } else {
                    section_range
                };

                if section_range.start > max_pos {
                    None
                } else if section_range.end < min_pos {
                    None
                } else {
                    Some((section_id, section_ui_state, section_range))
                }
            }};
        }

        let (finished_drag_offset, _) = ctx.ui_state.note_drag.handle_snapped(&mut ctx.ui_state.notes, |prepared| {
            let mut handle_note = |offset, note: &Note, note_id: Option<Id<Note>>| {
                let movement = prepared.movement().unwrap_or_default();

                let ui_data = note_id.map(|id| prepared.data_mut().get_mut_or_default(id));
                let selected = ui_data.as_ref().is_some_and(|ui_data| ui_data.selected);

                let mut note_range = note.range;
                let mut note_pitch = note.pitch;
                if selected {
                    note_range += movement.x as i64;
                    note_pitch += movement.y as i32;
                }
                let note_screen_range_x = pos_range_to_screen_range(note_range + offset);

                let note_y = note_pos_to_screen_pos((0, note_pitch)).y;
                let note_rect = Rect::from_x_y_ranges(
                    note_screen_range_x,
                    Rangef::new(note_y, note_y + self.units_per_pitch),
                );

                if selected || ctx.selection_rect.rect().intersects(note_rect) {
                    painter.rect(note_rect, Rounding::ZERO, Color32::DEBUG_COLOR, egui::Stroke::new(3.0, Color32::WHITE));
                }else{
                    painter.rect_filled(note_rect, Rounding::ZERO, Color32::DEBUG_COLOR);
                }
                if ctx.selection_rect.released_rect(self.id).is_some_and(|rect| rect.intersects(note_rect)) {
                    if let Some(ui_data) = ui_data {
                        ui_data.selected = true;
                    }
                }

                // if the note actually exists (it's not the currently drawn note)
                if let Some(note_id) = note_id {
                    // let ui_data = ctx.ui_state.notes.get(note_id);

                    const STRETCH_AREA_WIDTH: f32 = 4.0;
                    let note_interaction = ui.allocate_rect(note_rect.expand2(vec2(STRETCH_AREA_WIDTH / 2.0, 0.0)), egui::Sense::click_and_drag());
                    if note_interaction.dragged() {
                        prepared.set_scale((1.0 / self.units_per_tick, 1.0 / self.units_per_pitch));
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    prepared.process_interaction(note_interaction, note_id);
                }
            };

            for section in track.sections() {
                let Some((section_id, section_ui_data, section_range)) = get_section_render_data!(
                    section,
                    {
                        ctx.ui_state
                            .section_drag
                            .raw_movement_x()
                            .map(|m| snap_pos(m.round() as _, self.units_per_tick))
                    },
                    (ctx.ui_state.sections)
                ) else {
                    continue;
                };

                let section = ctx.state.sections.get_mut(section_id);

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
                for (_note_range, note_id) in section.notes() {
                    handle_note(
                        section_range.start,
                        ctx.state.notes.get(note_id),
                        Some(note_id),
                    );
                }
            }
            if let Some(ref note) = self.currently_drawn_note {
                handle_note(0, note, None);
            }
        }, |egui::Vec2 { x, y }| {
            vec2(snap_pos(x.round() as _, self.units_per_tick) as _, y.round())
        });
        if let Some(finished_drag_offset) = finished_drag_offset {
            let pos_offset = finished_drag_offset.x.round() as i64;
            let pitch_offset = finished_drag_offset.y.round() as i32;

            for (_section_range, section_id) in track.sections() {
                // let section_ui_data = ctx.ui_state.sections.get(section_id);
                // if section_ui_data.selected {
                let section = ctx.state.sections.get_mut(section_id);
                let mut notes_to_move = SmallVec::<[(Range, Id<Note>); 8]>::new();
                for (note_range, note_id) in section.notes() {
                    let note_ui_data = ctx.ui_state.notes.get_mut_or_default(note_id);
                    if note_ui_data.selected {
                        notes_to_move.push((note_range, note_id));
                    }
                }
                for &(note_range, note_id) in &notes_to_move {
                    section.move_note(
                        &mut ctx.state.notes,
                        note_range,
                        note_id,
                        note_range + pos_offset,
                        pitch_offset,
                    );
                }
                // }
            }
        }

        // Top area (for displaying sections, etc)
        const TOP_BAR_HEIGHT: f32 = 18.0;
        let top_bar_rect = Rect::from_x_y_ranges(
            screen_rect.x_range(),
            Rangef::new(screen_rect.top(), screen_rect.top() + TOP_BAR_HEIGHT),
        );
        let top_bar_interaction = ui.allocate_rect(top_bar_rect, egui::Sense::click_and_drag());

        painter.rect_filled(top_bar_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);
        painter.hline(
            top_bar_rect.x_range(),
            top_bar_rect.bottom(),
            ui.visuals().window_stroke,
        );

        // Bar indicators
        for bar in min_pos.div_floor(BEATS_PER_BAR * Range::UNITS_PER_BEAT)
            ..=max_pos.div_ceil(BEATS_PER_BAR * Range::UNITS_PER_BEAT)
        {
            let pos = bar * (BEATS_PER_BAR * Range::UNITS_PER_BEAT);
            painter.text(
                pos2(note_x_to_screen_x(pos as _), top_bar_rect.center().y),
                egui::Align2::CENTER_CENTER,
                bar.to_string(),
                egui::FontId::proportional(12.0),
                ui.visuals().widgets.hovered.text_color(),
            );
        }

        // Section headers

        let (finished_drag_offset, _) = ctx.ui_state.section_drag.handle_snapped(
            &mut ctx.ui_state.sections,
            |prepared| {
                for section in track.sections() {
                    let movement = prepared.movement_x();
                    let Some((section_id, section_ui_data, section_range)) =
                        get_section_render_data!(section, movement, (prepared.data_mut()))
                    else {
                        continue;
                    };

                    let section = ctx.state.sections.get_mut(section_id);

                    let section_screen_range_x = pos_range_to_screen_range(section_range);

                    let header_rect = Rect::from_x_y_ranges(
                        section_screen_range_x.expand(1.0),
                        top_bar_rect.y_range(),
                    );

                    let header_resp = ui.allocate_rect(header_rect, egui::Sense::click_and_drag());

                    painter.rect_filled(
                        header_rect,
                        Rounding {
                            nw: 6.0,
                            ne: 6.0,
                            sw: 0.0,
                            se: 0.0,
                        },
                        if section_ui_data.selected
                            || ctx.selection_rect.rect().intersects(header_rect)
                        {
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

                    if header_resp.dragged() {
                        prepared.set_scale((1.0 / self.units_per_tick, 0.0));
                    }
                    prepared.process_interaction(header_resp, section_id);
                }
            },
            |unsnapped| vec2(snap_pos(unsnapped.x as _, self.units_per_tick) as _, 0.0),
        );
        if let Some(finished_drag_offset) = finished_drag_offset {
            let finished_drag_offset = finished_drag_offset.x.round() as i64;

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

        let track = ctx.state.tracks.get_mut(track_id);

        if let Some(mouse_pos) = interaction.hover_pos() {
            let (pos, pitch) = screen_pos_to_note_pos(mouse_pos);
            let pos = snap_pos(pos, self.units_per_tick);
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
                        section.insert_note(&mut ctx.state.notes, note);
                    }

                    let screen_pos = note_pos_to_screen_pos((pos, pitch));
                    painter.vline(
                        screen_pos.x,
                        Rangef::new(screen_pos.y, screen_pos.y + self.units_per_pitch),
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

        interaction.context_menu(|ui| {
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

        if interaction.clicked() && single_thing_clicked.is_none() {
            single_thing_clicked = Some(Id::invalid());
        }
        if self.currently_drawn_note.is_none() || interaction.drag_released() {
            ctx.selection_rect.process_interaction(interaction, self.id);
        }

        if let Some(single_thing_clicked) = single_thing_clicked {
            for (&id, ui_data) in ctx.ui_state.sections.iter_mut() {
                if ui_data.selected && id.transmute() != single_thing_clicked {
                    ui_data.selected = false;
                }
            }
            for (&id, ui_data) in ctx.ui_state.notes.iter_mut() {
                if ui_data.selected && id.transmute() != single_thing_clicked {
                    ui_data.selected = false;
                }
            }
        }

        // Needle
        let needle_screen_x = painter.round_to_pixel(note_x_to_screen_x(ctx.state.needle_pos));
        if screen_rect.x_range().expand(8.0).contains(needle_screen_x) {
            let needle_stroke = ui.visuals().widgets.inactive.fg_stroke;
            // add a small amount to screen_rect.top() so the line doesn't poke through the triangle
            ui.painter().vline(
                needle_screen_x,
                Rangef::new(screen_rect.top() + 3.0, screen_rect.bottom()),
                needle_stroke,
            );
            let needle_top_pos = pos2(needle_screen_x, top_bar_rect.top());
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    pos2(needle_top_pos.x + 5.0, needle_top_pos.y),
                    pos2(needle_top_pos.x, needle_top_pos.y + 7.0),
                    pos2(needle_top_pos.x - 5.0, needle_top_pos.y),
                ],
                needle_stroke.color,
                egui::Stroke::NONE,
            ));
        }
        if let Some(pointer_pos) = top_bar_interaction.interact_pointer_pos() {
            ctx.state.needle_pos =
                snap_pos(screen_x_to_note_x(pointer_pos.x), self.units_per_tick) as _;
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
