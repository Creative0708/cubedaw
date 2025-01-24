use anyhow::Result;
use cubedaw_command::{note::NoteMove, section::SectionMove};
use cubedaw_lib::{Id, Note, Range, Section, Track};
use egui::{Color32, CursorIcon, Pos2, Rangef, Rect, Rounding, Vec2, pos2, vec2};

use crate::{
    app::Tab,
    command::{
        note::{UiNoteAddOrRemove, UiNoteSelect},
        section::{UiSectionAddOrRemove, UiSectionSelect},
    },
    state::ui::SectionUiState,
    widget::{SongViewer, SongViewerPrepared},
};

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track_id: Option<Id<Track>>,

    // Vertical zoom. Each note is this tall
    units_per_pitch: f32,

    last_mouse_position: (i64, i32),

    currently_drawn_note: Option<(i64, Note)>,
}

// Number of empty ticks to display on either side of the song
const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT as i64;

// Inclusive range. Standard 88-key keyboard, I think?
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

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
            if let Some(track_id) = self.track_id
                && ctx.state.tracks.has(track_id)
            {
                // TODO: allow persistent state in SongViewer
                SongViewer::new().ui(ctx, ui, |ctx, ui, prepared| {
                    self.pianoroll(ctx, ui, prepared)
                });
            } else {
                self.pianoroll_empty(ui);
            }
        });
        Ok(())
    }
}

impl PianoRollTab {
    fn pianoroll(
        &mut self,
        ctx: &mut crate::Context,
        ui: &mut egui::Ui,
        view: &SongViewerPrepared,
    ) {
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

        let anchor = view.anchor();

        let screen_rect = view.screen_rect;

        let screen_y_to_note_y = |screen_y: f32| -> i32 {
            MAX_NOTE_SHOWN - ((screen_y - anchor.y) / self.units_per_pitch) as i32
        };
        let screen_pos_to_note_pos = |screen_pos: Pos2| -> (i64, i32) {
            (
                view.screen_x_to_song_x(screen_pos.x),
                screen_y_to_note_y(screen_pos.y),
            )
        };
        let note_pos_to_screen_pos = |(pos, pitch): (i64, i32)| -> Pos2 {
            pos2(
                view.song_x_to_screen_x(pos),
                (MAX_NOTE_SHOWN - pitch) as f32 * self.units_per_pitch + anchor.y,
            )
        };

        let song_view_range = view.song_view_range;

        let min_pitch = screen_y_to_note_y(screen_rect.bottom());
        let max_pitch = screen_y_to_note_y(screen_rect.top());

        let bg_response = view.ui_background(
            ctx,
            ui,
            (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * self.units_per_pitch,
        );
        let snapped_hover_pos = bg_response.hover_pos().map(|hover_pos| {
            (
                view.input_screen_x_to_song_x(hover_pos.x),
                screen_y_to_note_y(hover_pos.y),
            )
        });

        // The horizontal "note lines"
        for row in min_pitch..=max_pitch {
            // TODO not hardcode this, probably after MVP
            if matches!(row % 12, 0 | 2 | 4 | 5 | 7 | 9 | 11) {
                let row_pos = note_pos_to_screen_pos((0, row)).y;
                ui.painter().rect_filled(
                    Rect::from_x_y_ranges(
                        screen_rect.x_range(),
                        Rangef::new(row_pos, row_pos + self.units_per_pitch),
                    ),
                    Rounding::ZERO,
                    ui.visuals().faint_bg_color,
                );
            }
        }

        let top_bar_interaction = view.ui_top_bar(ctx, ui);

        // Sections

        // TODO implement section colors
        const SECTION_COLOR: Color32 = Color32::from_rgb(145, 0, 235);

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
            |unsnapped| {
                vec2(
                    view.song_x_to_screen_x(view.input_screen_x_to_song_x(unsnapped.x)),
                    0.0,
                )
            },
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

                    if !song_view_range.intersects(section_range) {
                        continue;
                    }

                    let section_screen_range_x = view.song_range_to_screen_range(section_range);

                    let header_rect = Rect::from_x_y_ranges(
                        section_screen_range_x.expand(1.0),
                        view.top_bar_rect.y_range(),
                    );

                    let header_resp = ui
                        .allocate_rect(header_rect, egui::Sense::click_and_drag())
                        .on_hover_cursor(CursorIcon::Grab);

                    ui.painter().rect_filled(
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
                    ui.painter().text(
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
                        prepared.set_scale((1.0 / view.units_per_tick, 0.0));
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
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

                    let section_screen_range_x = view.song_range_to_screen_range(section_range);

                    ui.painter().rect_filled(
                        Rect::from_x_y_ranges(section_screen_range_x, screen_rect.y_range()),
                        Rounding::ZERO,
                        SECTION_COLOR
                            .gamma_multiply(0.2 * if section_ui.selected { 1.5 } else { 1.0 }),
                    );
                    ui.painter().vline(
                        section_screen_range_x.min,
                        screen_rect.y_range(),
                        section_stroke,
                    );
                    ui.painter().vline(
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
                result.should_deselect_everything || bg_response.clicked();
            let selection_changes = result.selection_changes;
            if should_deselect_everything {
                // TODO rename these
                for (track_id2, track_ui) in &ctx.ui_state.tracks {
                    for (section_id2, section_ui) in &track_ui.sections {
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
            |Vec2 { x, y }| {
                vec2(
                    view.song_x_to_screen_x(view.input_screen_x_to_song_x(x)),
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
                    let note_screen_range_x = view.song_range_to_screen_range(note_range);

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
                        ui.painter().rect(
                            note_rect,
                            Rounding::ZERO,
                            Color32::DEBUG_COLOR,
                            egui::Stroke::new(3.0, Color32::WHITE),
                        );
                    } else {
                        ui.painter()
                            .rect_filled(note_rect, Rounding::ZERO, Color32::DEBUG_COLOR);
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
                        let note_interaction = ui
                            .allocate_rect(
                                note_rect.expand2(vec2(STRETCH_AREA_WIDTH / 2.0, 0.0)),
                                egui::Sense::click_and_drag(),
                            )
                            .on_hover_cursor(CursorIcon::Grab);
                        if note_interaction.dragged() {
                            prepared.set_scale((
                                1.0 / view.units_per_tick,
                                -1.0 / self.units_per_pitch,
                            ));
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
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
                result.should_deselect_everything || bg_response.clicked();
            let selection_changes = result.selection_changes;
            if should_deselect_everything {
                // TODO rename these
                for (track_id2, track_ui) in &ctx.ui_state.tracks {
                    for (section_id2, section_ui) in &track_ui.sections {
                        for (note_id2, note_ui) in &section_ui.notes {
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

                for (section_id, section_ui) in &track_ui.sections {
                    for (note_id, note_ui) in &section_ui.notes {
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
            if bg_response.hovered() && ui.input(|i| i.modifiers.ctrl) {
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

                // that little vertical line that shows you where the next note would be drawn
                if let Some((pos, mut pitch)) = snapped_hover_pos {
                    if let Some((_, ref note)) = self.currently_drawn_note {
                        pitch = note.pitch;
                    }

                    let screen_pos = note_pos_to_screen_pos((pos, pitch));
                    ui.painter().vline(
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

        bg_response.context_menu(|ui| {
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
        if self.currently_drawn_note.is_none() || bg_response.drag_stopped() {
            ctx.ephemeral_state
                .selection_rect
                .process_interaction(&bg_response, self.id);
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

        if ctx.focused_tab() == Some(self.id)
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(egui::Modifiers::default(), egui::Key::Delete))
        {
            // we should really store the selected sections/notes/whatever
            // so we don't have to iterate over _every_ note in order to find the selected ones
            for (track_id2, track_ui) in &ctx.ui_state.tracks {
                for (section_id2, section_ui) in &track_ui.sections {
                    for (note_id2, note_ui) in &section_ui.notes {
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

        view.ui_playhead(ctx, ui);
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
