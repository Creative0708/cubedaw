use anyhow::Result;
use cubedaw_command::{note::NoteMove, section::SectionMove};
use cubedaw_lib::{Id, Note, PreciseSongPos, Range, Section, Track};
use egui::{Color32, Pos2, Rangef, Rect, Rounding, pos2, vec2};

use crate::{
    app::Tab,
    command::{
        misc::UiSetPlayhead,
        note::{UiNoteAddOrRemove, UiNoteSelect},
        section::{UiSectionAddOrRemove, UiSectionSelect},
    },
    state::ui::SectionUiState,
};

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track_id: Option<Id<Track>>,

    // Vertical zoom. Each note is this tall
    units_per_pitch: f32,
    // Horizontal zoom. Each tick is this wide
    units_per_tick: f32,

    last_mouse_position: (i64, i32),

    currently_drawn_note: Option<(i64, Note)>,
}

// Number of empty ticks to display on either side of the song
const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT as i64;
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

fn snap_pos(pos: i64, units_per_tick: f32) -> i64 {
    let step = ((Range::UNITS_PER_BEAT as f32 / units_per_tick * 0.05).min(256.0) as u32)
        .next_power_of_two() as i64;

    (pos + step / 2).div_floor(step) * step
}

impl crate::Screen for PianoRollTab {
    fn create(state: &cubedaw_lib::State, ui_state: &crate::UiState) -> Self {
        Self {
            id: Id::arbitrary(),

            track_id: ui_state.get_single_selected_track().and_then(|track_id| {
                state
                    .tracks
                    .force_get(track_id)
                    .inner
                    .is_section()
                    .then_some(track_id)
            }),

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

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) -> Result<()> {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().show_viewport(ui, |ui, viewport| {
                if let Some(track_id) = self.track_id
                    && ctx.state.tracks.has(track_id)
                {
                    self.pianoroll(ctx, ui, viewport);
                } else {
                    self.pianoroll_empty(ui);
                }
            })
        });
        Ok(())
    }
}

impl PianoRollTab {
    fn pianoroll(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, viewport: Rect) {
        let Some(track_id) = self.track_id else {
            unreachable!()
        };
        let Some(outer_track) = ctx.state.tracks.get(track_id) else {
            unreachable!()
        };
        let Some(track) = outer_track.inner.section() else {
            unreachable!();
        };
        let Some(track_ui) = ctx.ui_state.tracks.get(track_id) else {
            unreachable!()
        };

        let max_rect = ui.max_rect();
        let top_left = max_rect.left_top().to_vec2();
        let painter = ui.painter_at(viewport.translate(top_left));

        let screen_rect = viewport.translate(top_left);

        painter.rect_filled(screen_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);

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
        let precise_x_to_screen_x = |pos: PreciseSongPos| -> f32 {
            (pos - PreciseSongPos::from_song_pos(ctx.state.song_boundary.start - SONG_PADDING))
                .to_song_pos_f32()
                * self.units_per_tick
                + top_left.x
        };
        let note_x_to_screen_x = |pos: i64| -> f32 {
            (pos - (ctx.state.song_boundary.start - SONG_PADDING)) as f32 * self.units_per_tick
                + top_left.x
        };
        let note_pos_to_screen_pos = |(pos, pitch): (i64, i32)| -> Pos2 {
            pos2(
                note_x_to_screen_x(pos),
                (pitch - MIN_NOTE_SHOWN) as f32 * self.units_per_pitch + top_left.y,
            )
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
        let pos_view_range = Range {
            start: min_pos,
            end: max_pos,
        };

        let (_, response) = ui.allocate_exact_size(
            vec2(
                (ctx.state.song_boundary.length() + SONG_PADDING * 2) as f32 * self.units_per_tick,
                (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * self.units_per_pitch,
            ),
            egui::Sense::click_and_drag(),
        );
        let snapped_hover_pos = response.hover_pos().map(|pos2| {
            let (pos, pitch) = screen_pos_to_note_pos(pos2);
            (snap_pos(pos, self.units_per_tick), pitch)
        });

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
                    ui.visuals().faint_bg_color,
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
            let stroke = if pos % (BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64) == 0 {
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

        // Sections
        struct RenderedSection<'a> {
            id: Id<Section>,
            range: Range,
            state: &'a Section,
            ui_state: &'a SectionUiState,
        }
        let mut rendered_sections: Vec<RenderedSection> = Vec::new();

        let result = ctx.ephemeral_state.drag.handle_snapped(
            Id::new("section"),
            |unsnapped| vec2(snap_pos(unsnapped.x as _, self.units_per_tick) as _, 0.0),
            |prepared| {
                for (section_range, section_id, section) in track.sections() {
                    let section_ui = track_ui.sections.force_get(section_id);

                    let section_range = if let Some(section_drag) = prepared.movement_x() {
                        if section_ui.selected {
                            section_range + section_drag as i64
                        } else {
                            section_range
                        }
                    } else {
                        section_range
                    };

                    if !pos_view_range.intersects(section_range) {
                        continue;
                    }

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
                        if section_ui.selected
                            || ctx
                                .ephemeral_state
                                .selection_rect
                                .rect()
                                .intersects(header_rect)
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
                        if section_ui.selected {
                            &ui.visuals().widgets.hovered
                        } else {
                            ui.visuals().widgets.style(&header_resp)
                        }
                        .text_color(),
                    );

                    if header_resp.dragged() {
                        prepared.set_scale((1.0 / self.units_per_tick, 0.0));
                    }
                    prepared.process_interaction(
                        section_id.cast(),
                        &header_resp,
                        (track_id, section_id),
                        section_ui.selected,
                    );

                    let section_stroke = egui::Stroke::new(
                        2.0,
                        SECTION_COLOR
                            .gamma_multiply(0.5 * if section_ui.selected { 1.5 } else { 1.0 }),
                    );

                    let section_screen_range_x = pos_range_to_screen_range(section_range);

                    painter.rect_filled(
                        Rect::from_x_y_ranges(section_screen_range_x, screen_rect.y_range()),
                        Rounding::ZERO,
                        SECTION_COLOR
                            .gamma_multiply(0.2 * if section_ui.selected { 1.5 } else { 1.0 }),
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

                    rendered_sections.push(RenderedSection {
                        id: section_id,
                        range: section_range,
                        state: section,
                        ui_state: section_ui,
                    });
                }
            },
        );
        {
            let should_deselect_everything =
                result.should_deselect_everything || response.clicked();
            let selection_changes = result.selection_changes;
            if should_deselect_everything {
                // TODO rename these
                for (&track_id2, track_ui) in &ctx.ui_state.tracks {
                    for (&section_id2, section_ui) in &track_ui.sections {
                        if section_ui.selected
                            && selection_changes.get(&(track_id2, section_id2)).copied()
                                != Some(true)
                        {
                            ctx.tracker
                                .add(UiSectionSelect::new(track_id2, section_id2, false));
                        }
                    }
                }
                for (&(track_id, section_id), &selected) in &selection_changes {
                    if selected
                        && !ctx
                            .ui_state
                            .tracks
                            .get(track_id)
                            .and_then(|t| t.sections.get(section_id))
                            .is_some_and(|n| n.selected)
                    {
                        ctx.tracker
                            .add(UiSectionSelect::new(track_id, section_id, true));
                    }
                }
            } else {
                for (&(track_id, section_id), &selected) in &selection_changes {
                    ctx.tracker
                        .add(UiSectionSelect::new(track_id, section_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                let finished_drag_offset = finished_drag_offset.x as i64;
                for (section_range, section_id, _section) in track.sections() {
                    let section_ui = track_ui.sections.force_get(section_id);
                    if section_ui.selected {
                        ctx.tracker.add(SectionMove::new(
                            track_id,
                            section_range,
                            section_range.start + finished_drag_offset,
                        ));
                    }
                }
            }
        }

        // Handle notes

        let result = ctx.ephemeral_state.drag.handle_snapped(
            Id::new("notes"),
            |egui::Vec2 { x, y }| {
                vec2(
                    snap_pos(x.round() as _, self.units_per_tick) as _,
                    y.round(),
                )
            },
            |prepared| {
                let mut handle_note = |prepared: &mut crate::util::Prepared<'_, _, _>,
                                       relative_start_pos: i64,
                                       note: &Note,
                                       section_and_note_id: Option<(Id<Section>, Id<Note>)>,
                                       is_selected: bool| {
                    let movement = prepared.movement().unwrap_or_default();

                    let mut note_range = note.range_with(relative_start_pos);
                    let mut note_pitch = note.pitch;
                    if is_selected {
                        note_range += movement.x as i64;
                        note_pitch += movement.y as i32;
                    }
                    let note_screen_range_x = pos_range_to_screen_range(note_range);

                    let note_y = note_pos_to_screen_pos((0, note_pitch)).y;
                    let note_rect = Rect::from_x_y_ranges(
                        note_screen_range_x,
                        Rangef::new(note_y, note_y + self.units_per_pitch),
                    );

                    if is_selected
                        || ctx
                            .ephemeral_state
                            .selection_rect
                            .rect()
                            .intersects(note_rect)
                    {
                        painter.rect(
                            note_rect,
                            Rounding::ZERO,
                            Color32::DEBUG_COLOR,
                            egui::Stroke::new(3.0, Color32::WHITE),
                        );
                    } else {
                        painter.rect_filled(note_rect, Rounding::ZERO, Color32::DEBUG_COLOR);
                    }
                    if ctx
                        .ephemeral_state
                        .selection_rect
                        .released_rect(self.id)
                        .is_some_and(|rect| rect.intersects(note_rect))
                    {
                        if let Some((section_id, note_id)) = section_and_note_id {
                            ctx.tracker
                                .add(UiNoteSelect::new(track_id, section_id, note_id, true));
                        }
                    }

                    // if the note actually exists (it's not the currently drawn note)
                    if let Some((section_id, note_id)) = section_and_note_id {
                        // let ui_data = ctx.ui_state.notes.get(note_id);

                        const STRETCH_AREA_WIDTH: f32 = 4.0;
                        let note_interaction = ui.allocate_rect(
                            note_rect.expand2(vec2(STRETCH_AREA_WIDTH / 2.0, 0.0)),
                            egui::Sense::click_and_drag(),
                        );
                        if note_interaction.dragged() {
                            prepared
                                .set_scale((1.0 / self.units_per_tick, 1.0 / self.units_per_pitch));
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }
                        prepared.process_interaction(
                            note_id.cast(),
                            &note_interaction,
                            (track_id, section_id, note_id),
                            is_selected,
                        );
                    }
                };

                for RenderedSection {
                    id: section_id,
                    range,
                    state: section,
                    ui_state: section_ui,
                } in rendered_sections
                {
                    // Notes
                    for (note_start, note_id, note) in section.notes() {
                        handle_note(
                            prepared,
                            range.start + note_start,
                            note,
                            Some((section_id, note_id)),
                            section_ui.notes.force_get(note_id).selected,
                        );
                    }
                }
                if let Some((start_pos, ref note)) = self.currently_drawn_note {
                    handle_note(prepared, start_pos, note, None, true);
                }
            },
        );
        {
            let should_deselect_everything =
                result.should_deselect_everything || response.clicked();
            let selection_changes = result.selection_changes;
            if should_deselect_everything {
                // TODO rename these
                for (&track_id2, track_ui) in &ctx.ui_state.tracks {
                    for (&section_id2, section_ui) in &track_ui.sections {
                        for (&note_id2, note_ui) in &section_ui.notes {
                            if note_ui.selected
                                && selection_changes
                                    .get(&(track_id2, section_id2, note_id2))
                                    .copied()
                                    != Some(true)
                            {
                                ctx.tracker.add(UiNoteSelect::new(
                                    track_id2,
                                    section_id2,
                                    note_id2,
                                    false,
                                ));
                            }
                        }
                    }
                }
                for (&(track_id, section_id, note_id), &selected) in &selection_changes {
                    // only add a command when a note should be selected and isn't currently selected.
                    // the deselection is handled in the for loop before this one
                    if selected
                        && !ctx
                            .ui_state
                            .tracks
                            .get(track_id)
                            .and_then(|t| t.sections.get(section_id))
                            .and_then(|s| s.notes.get(note_id))
                            .is_some_and(|n| n.selected)
                    {
                        ctx.tracker
                            .add(UiNoteSelect::new(track_id, section_id, note_id, true));
                    }
                }
            } else {
                for (&(track_id, section_id, note_id), &selected) in &selection_changes {
                    ctx.tracker
                        .add(UiNoteSelect::new(track_id, section_id, note_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                let pos_offset = finished_drag_offset.x.round() as i64;
                let pitch_offset = finished_drag_offset.y.round() as i32;

                for (&section_id, section_ui) in &track_ui.sections {
                    for (&note_id, note_ui) in &section_ui.notes {
                        if note_ui.selected {
                            ctx.tracker.add(NoteMove::new(
                                track_id,
                                section_id,
                                note_id,
                                pos_offset,
                                pitch_offset,
                            ));
                        }
                    }
                }
            }
        }

        {
            // Handle drawn note
            if response.hovered() && ui.input(|i| i.modifiers.ctrl) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                    && let Some((pos, pitch)) = snapped_hover_pos
                {
                    self.currently_drawn_note = Some((pos, Note::new(0, pitch)));
                }
                if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                    if let Some((start_pos, note)) = self.currently_drawn_note.take() {
                        let (section_range, section_id) = match track.section_at(start_pos) {
                            Some(data) => data,
                            None => {
                                let section_id = Id::arbitrary();
                                let section_range = Range::surrounding_pos(start_pos);
                                let section = Section::empty(
                                    "New Section".into(),
                                    section_range.length() as _,
                                );
                                track.check_overlap_with(section_range);
                                ctx.tracker.add(UiSectionAddOrRemove::addition(
                                    section_id,
                                    section_range.start,
                                    section,
                                    track_id,
                                ));
                                (section_range, section_id)
                            }
                        };
                        ctx.tracker.add(UiNoteAddOrRemove::addition(
                            Id::arbitrary(),
                            track_id,
                            section_id,
                            start_pos - section_range.start,
                            note,
                        ));
                    }
                } else if let Some((starting_pos, ref mut note)) = self.currently_drawn_note
                    && let Some((pos, _)) = snapped_hover_pos
                {
                    note.length = (pos - starting_pos).max(0) as _;
                }

                if let Some((pos, mut pitch)) = snapped_hover_pos {
                    if let Some((_, ref note)) = self.currently_drawn_note {
                        pitch = note.pitch;
                    }

                    let screen_pos = note_pos_to_screen_pos((pos, pitch));
                    painter.vline(
                        screen_pos.x,
                        Rangef::new(screen_pos.y, screen_pos.y + self.units_per_pitch),
                        if self.currently_drawn_note.is_some() {
                            ui.visuals().widgets.active
                        } else {
                            ui.visuals().widgets.hovered
                        }
                        .fg_stroke,
                    );
                }
            } else {
                self.currently_drawn_note = None;
            }
        }

        // Bar indicators
        for bar in min_pos.div_floor(BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64)
            ..=max_pos.div_ceil(BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64)
        {
            let pos = bar * (BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64);
            painter.text(
                pos2(note_x_to_screen_x(pos as _), top_bar_rect.center().y),
                egui::Align2::CENTER_CENTER,
                bar.to_string(),
                egui::FontId::proportional(12.0),
                ui.visuals().widgets.hovered.text_color(),
            );
        }

        let finished_drag_offset = result.movement;

        if let Some(finished_drag_offset) = finished_drag_offset {
            let finished_drag_offset = finished_drag_offset.x.round() as i64;

            for (section_range, section_id, _section) in track.sections() {
                if track_ui.sections.force_get(section_id).selected {
                    ctx.tracker.add(SectionMove::new(
                        track_id,
                        section_range,
                        section_range.start + finished_drag_offset,
                    ));
                }
            }
            track.check_overlap();
        }

        response.context_menu(|ui| {
            if ui.button("Add section").clicked() {
                let section_range = Range::surrounding_pos(self.last_mouse_position.0);
                ctx.tracker.add(UiSectionAddOrRemove::addition(
                    Id::arbitrary(),
                    section_range.start,
                    Section::empty("New Section".into(), section_range.length() as _),
                    track_id,
                ));
                ui.close_menu();
            }
        });

        // if interaction.clicked() && single_thing_clicked.is_none() {
        //     single_thing_clicked = Some(None);
        // }
        if self.currently_drawn_note.is_none() || response.drag_stopped() {
            ctx.ephemeral_state
                .selection_rect
                .process_interaction(&response, self.id);
        }

        // if let Some(single_thing_clicked) = single_thing_clicked {
        //     for (&id, ui_data) in track_ui.sections.iter() {
        //         if ui_data.selected && Some(id.transmute()) != single_thing_clicked {
        //             ctx.tracker.add(UiSectionSelect::new(id, false));
        //         }
        //     }
        //     for (&id, ui_data) in track_ui.notes.iter() {
        //         if ui_data.selected && Some(id.transmute()) != single_thing_clicked {
        //             ctx.tracker.add(UiNoteSelect::new(id, false));
        //         }
        //     }
        // }

        // playhead
        let playhead_screen_x =
            painter.round_to_pixel(ctx.currently_playing_playhead_pos().map_or_else(
                || note_x_to_screen_x(ctx.ui_state.playhead_pos),
                precise_x_to_screen_x,
            ));
        if screen_rect
            .x_range()
            .expand(8.0)
            .contains(playhead_screen_x)
        {
            let playhead_stroke = ui.visuals().widgets.inactive.fg_stroke;
            // add a small amount to screen_rect.top() so the line doesn't poke through the triangle
            ui.painter().vline(
                playhead_screen_x,
                Rangef::new(screen_rect.top() + 3.0, screen_rect.bottom()),
                playhead_stroke,
            );
            let playhead_top_pos = pos2(playhead_screen_x, top_bar_rect.top());
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    pos2(playhead_top_pos.x + 5.0, playhead_top_pos.y),
                    pos2(playhead_top_pos.x, playhead_top_pos.y + 7.0),
                    pos2(playhead_top_pos.x - 5.0, playhead_top_pos.y),
                ],
                playhead_stroke.color,
                egui::Stroke::NONE,
            ));
        }
        if top_bar_interaction.contains_pointer()
            && let Some(pointer_pos) = ui.input(|i| {
                if i.pointer.primary_clicked() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            })
        {
            ctx.tracker.add_weak(UiSetPlayhead::new(snap_pos(
                screen_x_to_note_x(pointer_pos.x),
                self.units_per_tick,
            ) as _));
        }

        if ctx.focused_tab() == Some(self.id)
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(egui::Modifiers::default(), egui::Key::Delete))
        {
            // we should really store the selected sections/notes/whatever
            // so we don't have to iterate over _every_ note in order to find the selected ones
            for (&track_id2, track_ui) in &ctx.ui_state.tracks {
                for (&section_id2, section_ui) in &track_ui.sections {
                    for (&note_id2, note_ui) in &section_ui.notes {
                        if note_ui.selected {
                            ctx.tracker.add(UiNoteAddOrRemove::removal(
                                track_id2,
                                section_id2,
                                note_id2,
                            ));
                        }
                    }
                }
            }
        }

        if let Some(hover_pos) = snapped_hover_pos {
            self.last_mouse_position = hover_pos;
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

    pub fn select_track(&mut self, track_id: Option<Id<Track>>) {
        self.track_id = track_id;
        self.currently_drawn_note = None;
    }
}
