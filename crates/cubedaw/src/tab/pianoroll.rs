use std::ops;

use anyhow::Result;
use cubedaw_lib::{Clip, Id, Note, Range, Track};
use egui::{
    Color32, CornerRadius, CursorIcon, Pos2, Rangef, Rect, Response, Stroke, StrokeKind, pos2, vec2,
};

use crate::{
    app::Tab,
    command::{
        clip::UiClipAddOrRemove,
        note::{UiNoteAddOrRemove, UiNoteSelect},
    },
    context::UiStateTracker,
    state::ui::{ClipUiState, TrackUiState},
    tab::track::Track2DPos,
    util::{Select, SelectionRect},
    widget::{SongViewer, SongViewerPrepared},
};

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track_id: Option<Id<Track>>,

    // Vertical zoom. Each note is this tall
    units_per_pitch: f32,

    currently_drawn_note: Option<(i64, Note)>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Note2DPos {
    pub time: i64,
    pub pitch: i32,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Note2DOffset {
    pub time: i64,
    pub pitch: i32,
}
impl ops::Sub for Note2DPos {
    type Output = Note2DOffset;
    fn sub(self, rhs: Self) -> Self::Output {
        Note2DOffset {
            time: self.time - rhs.time,
            pitch: self.pitch - rhs.pitch,
        }
    }
}

// Inclusive range. Standard 88-key keyboard, I think?
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

impl crate::Screen for PianoRollTab {
    fn create(_state: &cubedaw_lib::State, ui_state: &crate::UiState) -> Self {
        Self {
            id: Id::arbitrary(),

            track_id: ui_state.get_single_selected_track(),

            units_per_pitch: 16.0,

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
            SongViewer::new().ui(ctx, ui, |ctx, ui, view| {
                match Prepared::start(ui, ctx, view, self) {
                    Some(mut prepared) => {
                        let rendered_clips = prepared.handle_clips(ui, ctx);
                        prepared.handle_notes(ui, ctx, self, &rendered_clips);
                        prepared.handle_drawn_note(ui, ctx, self);
                    }
                    None => {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.label("No track selected");
                            },
                        );
                    }
                }
            });
        });
        Ok(())
    }
}

impl PianoRollTab {
    pub fn select_track(&mut self, track_id: Option<Id<Track>>) {
        self.track_id = track_id;
        self.currently_drawn_note = None;
    }
}

// Clips
struct RenderedClip<'a> {
    id: Id<Clip>,
    range: Range,
    state: &'a Clip,
    ui_state: &'a ClipUiState,
}

struct Prepared<'ctx, 'arg> {
    track_id: Id<Track>,
    track: &'ctx Track,
    track_ui: &'ctx TrackUiState,
    tab_id: Id<Tab>,

    view: &'arg SongViewerPrepared<'arg>,
    bg_response: Response,
    snapped_hover_pos: Option<Note2DPos>,

    ntspc: NoteToScreenPosCalculator,
}
#[derive(Debug, Clone, Copy)]
struct NoteToScreenPosCalculator {
    anchor: Pos2,
    units_per_pitch: f32,
}
impl NoteToScreenPosCalculator {
    fn screen_y_to_note_y(&self, screen_y: f32) -> i32 {
        MAX_NOTE_SHOWN - ((screen_y - self.anchor.y) / self.units_per_pitch) as i32
    }
    fn note_y_to_screen_y(&self, pitch: i32) -> f32 {
        (MAX_NOTE_SHOWN - pitch) as f32 * self.units_per_pitch + self.anchor.y
    }
}

impl<'ctx, 'arg> Prepared<'ctx, 'arg> {
    fn start(
        ui: &mut egui::Ui,
        ctx: &mut crate::Context<'ctx>,
        view: &'arg SongViewerPrepared<'arg>,
        tab: &mut PianoRollTab,
    ) -> Option<Self> {
        let track_id = tab.track_id?;

        let bg_response = view.ui_background(
            ctx,
            ui,
            (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * tab.units_per_pitch,
        );

        view.ui_top_bar(ctx, ui);

        let ntspc = NoteToScreenPosCalculator {
            anchor: view.anchor(),
            units_per_pitch: tab.units_per_pitch,
        };

        Some(Self {
            track_id,
            track: ctx.state.tracks.get(track_id)?,
            track_ui: ctx.ui_state.tracks.force_get(track_id),
            tab_id: tab.id,

            view,
            snapped_hover_pos: bg_response.hover_pos().map(|hover_pos| Note2DPos {
                time: view.input_screen_x_to_song_x(hover_pos.x),
                pitch: ntspc.screen_y_to_note_y(hover_pos.y),
            }),
            bg_response,
            ntspc,
        })
    }

    fn units_per_pitch(&self) -> f32 {
        self.ntspc.units_per_pitch
    }

    fn handle_clips(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
    ) -> Vec<RenderedClip<'ctx>> {
        let Self {
            track_id,
            track,
            track_ui,
            view,
            ref bg_response,
            ..
        } = *self;

        let SongViewerPrepared {
            screen_rect,
            song_view_range,
            top_bar_rect,
            ..
        } = *view;

        // TODO implement clip colors
        const SECTION_COLOR: Color32 = Color32::from_rgb(145, 0, 235);

        let mut rendered_clips: Vec<RenderedClip> = Vec::new();

        ctx.ephemeral_state.clip_drag.handle(
            // strictly speaking we should use the current track's index instead of 0 but it doesn't matter anyways
            |unsnapped| Track2DPos {
                time: view.input_screen_x_to_song_x(unsnapped.x),
                idx: 0,
            },
            |prepared| {
                if bg_response.clicked() {
                    prepared.deselect_all();
                }

                for (clip_range, clip_id, clip) in track.clips() {
                    let clip_ui = track_ui.clips.force_get(clip_id);

                    let clip_range = if let Some(clip_drag) = prepared.movement() {
                        match clip_ui.select {
                            Select::Select => clip_range + clip_drag.time,
                            Select::Deselect => clip_range,
                        }
                    } else {
                        clip_range
                    };

                    if prepared.dragged_thing() != Some(clip_id.cast())
                        && !song_view_range.intersects(clip_range)
                    {
                        continue;
                    }

                    let clip_screen_range_x = view.song_range_to_screen_range(clip_range);

                    let header_rect = Rect::from_x_y_ranges(
                        clip_screen_range_x.expand(1.0),
                        top_bar_rect.y_range(),
                    );

                    let is_effectively_selected = clip_ui.select.is()
                        || ctx
                            .ephemeral_state
                            .selection_rect
                            .rect()
                            .intersects(header_rect);

                    let header_resp = ui
                        .allocate_rect(header_rect, egui::Sense::click_and_drag())
                        .on_hover_cursor(CursorIcon::Grab);

                    ui.painter().rect_filled(
                        header_rect,
                        CornerRadius {
                            nw: 6,
                            ne: 6,
                            sw: 0,
                            se: 0,
                        },
                        if is_effectively_selected {
                            SECTION_COLOR.gamma_multiply(0.7)
                        } else {
                            SECTION_COLOR.gamma_multiply(0.5)
                        },
                    );

                    let padding = header_rect.height() * 0.5;
                    ui.painter().text(
                        pos2(header_rect.left() + padding, header_rect.top() + padding),
                        egui::Align2::LEFT_CENTER,
                        &clip.name,
                        egui::FontId::proportional(12.0),
                        if is_effectively_selected {
                            &ui.visuals().widgets.hovered
                        } else {
                            ui.visuals().widgets.style(&header_resp)
                        }
                        .text_color(),
                    );

                    if header_resp.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    }
                    prepared.process_interaction(
                        clip_id.cast(),
                        &header_resp,
                        (track_id, clip_id),
                        clip_ui.select,
                    );

                    let clip_stroke = egui::Stroke::new(
                        2.0,
                        SECTION_COLOR
                            .gamma_multiply(0.5 * if is_effectively_selected { 1.5 } else { 1.0 }),
                    );

                    let clip_screen_range_x = view.song_range_to_screen_range(clip_range);

                    ui.painter().rect_filled(
                        Rect::from_x_y_ranges(clip_screen_range_x, screen_rect.y_range()),
                        CornerRadius::ZERO,
                        SECTION_COLOR
                            .gamma_multiply(0.2 * if is_effectively_selected { 1.5 } else { 1.0 }),
                    );
                    ui.painter()
                        .vline(clip_screen_range_x.min, screen_rect.y_range(), clip_stroke);
                    ui.painter()
                        .vline(clip_screen_range_x.max, screen_rect.y_range(), clip_stroke);

                    rendered_clips.push(RenderedClip {
                        id: clip_id,
                        range: clip_range,
                        state: clip,
                        ui_state: clip_ui,
                    });
                }
            },
        );

        rendered_clips
    }

    fn handle_note(
        &mut self,
        ui: &mut egui::Ui,
        selection_rect: &mut SelectionRect,
        tracker: &mut UiStateTracker,

        prepared: &mut crate::util::Prepared<
            (Id<Track>, Id<Clip>, Id<Note>),
            impl Fn(Pos2) -> Note2DPos,
        >,
        relative_start_pos: i64,
        note: &Note,
        note_path: Option<(Id<Clip>, Id<Note>)>,
        select: Select,
    ) {
        let Self {
            track_id,
            tab_id,

            view,
            ntspc,
            ..
        } = *self;

        let Note2DOffset {
            time: movement_time,
            pitch: movement_pitch,
        } = prepared.movement().unwrap_or_default();

        let mut note_range = note.range_with(relative_start_pos);
        let mut note_pitch = note.pitch;
        if select.is() {
            note_range += movement_time;
            note_pitch += movement_pitch;
        }
        let note_screen_range_x = view.song_range_to_screen_range(note_range);

        let note_y = ntspc.note_y_to_screen_y(note_pitch);
        let note_rect = Rect::from_x_y_ranges(
            note_screen_range_x,
            Rangef::new(note_y, note_y + self.units_per_pitch()),
        );

        if select.is() || selection_rect.rect().intersects(note_rect) {
            ui.painter().rect(
                note_rect,
                CornerRadius::ZERO,
                Color32::DEBUG_COLOR,
                Stroke::new(2.0, Color32::WHITE),
                StrokeKind::Outside,
            );
        } else {
            ui.painter()
                .rect_filled(note_rect, CornerRadius::ZERO, Color32::DEBUG_COLOR);
        }
        if selection_rect
            .released_rect(tab_id)
            .is_some_and(|rect| rect.intersects(note_rect))
        {
            if let Some((clip_id, note_id)) = note_path {
                tracker.add(UiNoteSelect::new(
                    track_id,
                    clip_id,
                    note_id,
                    Select::Select,
                ));
            }
        }

        // if the note actually exists (it's not the currently drawn note)
        if let Some((clip_id, note_id)) = note_path {
            // let ui_data = ctx.ui_state.notes.get(note_id);

            const STRETCH_AREA_WIDTH: f32 = 4.0;
            let note_interaction = ui
                .allocate_rect(
                    note_rect.expand2(vec2(STRETCH_AREA_WIDTH / 2.0, 0.0)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(CursorIcon::Grab);
            if note_interaction.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            prepared.process_interaction(
                note_id.cast(),
                &note_interaction,
                (track_id, clip_id, note_id),
                select,
            );
        }
    }
    fn handle_notes(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        tab: &mut PianoRollTab,
        rendered_clips: &[RenderedClip],
    ) {
        let Self { view, ntspc, .. } = *self;

        ctx.ephemeral_state.note_drag.handle(
            move |Pos2 { x, y }| Note2DPos {
                time: view.input_screen_x_to_song_x(x),
                pitch: ntspc.screen_y_to_note_y(y),
            },
            |prepared| {
                if self.bg_response.clicked() {
                    prepared.deselect_all();
                }

                for &RenderedClip {
                    id: clip_id,
                    range,
                    state: clip,
                    ui_state: clip_ui,
                } in rendered_clips
                {
                    // Notes
                    for (note_start, note_id, note) in clip.notes() {
                        self.handle_note(
                            ui,
                            &mut ctx.ephemeral_state.selection_rect,
                            &mut ctx.tracker,
                            prepared,
                            range.start + note_start,
                            note,
                            Some((clip_id, note_id)),
                            clip_ui.notes.force_get(note_id).select,
                        );
                    }
                }
                if let Some((start_pos, ref note)) = tab.currently_drawn_note {
                    self.handle_note(
                        ui,
                        &mut ctx.ephemeral_state.selection_rect,
                        &mut ctx.tracker,
                        prepared,
                        start_pos,
                        note,
                        None,
                        Select::Select,
                    );
                }
            },
        );
    }
    fn handle_drawn_note(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        tab: &mut PianoRollTab,
    ) {
        let Self {
            track_id,
            track,

            view,
            ref bg_response,
            snapped_hover_pos,

            ntspc,
            ..
        } = *self;
        if bg_response.hovered() && ui.input(|i| i.modifiers.ctrl) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary))
                && let Some(pos) = snapped_hover_pos
            {
                tab.currently_drawn_note = Some((pos.time, Note::new(0, pos.pitch)));
            }
            if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                if let Some((start_pos, note)) = tab.currently_drawn_note.take() {
                    let (clip_range, clip_id) = match track.clip_at(start_pos) {
                        Some(data) => data,
                        None => {
                            let clip_id = Id::arbitrary();
                            let clip_range = Range::surrounding_pos(start_pos);
                            let clip = Clip::empty("New Clip".into(), clip_range.length() as _);
                            track.check_overlap_with(clip_range);
                            ctx.tracker.add(UiClipAddOrRemove::addition(
                                clip_id,
                                clip_range.start,
                                clip,
                                track_id,
                            ));
                            (clip_range, clip_id)
                        }
                    };
                    ctx.tracker.add(UiNoteAddOrRemove::addition(
                        Id::arbitrary(),
                        track_id,
                        clip_id,
                        start_pos - clip_range.start,
                        note,
                    ));
                }
            } else if let Some((starting_pos, ref mut note)) = tab.currently_drawn_note
                && let Some(pos) = snapped_hover_pos
            {
                note.length = (pos.time - starting_pos).max(0) as _;
            }

            // that little vertical line that shows you where the next note would be drawn
            if let Some(Note2DPos { time, mut pitch }) = snapped_hover_pos {
                if let Some((_, ref note)) = tab.currently_drawn_note {
                    pitch = note.pitch;
                }

                let stroke = if tab.currently_drawn_note.is_some() {
                    ui.visuals().widgets.active
                } else {
                    ui.visuals().widgets.hovered
                }
                .fg_stroke;

                let screen_x = view.song_x_to_screen_x(time);
                let screen_y = ntspc.note_y_to_screen_y(pitch);
                ui.painter().vline(
                    screen_x,
                    Rangef::new(screen_y, screen_y + self.units_per_pitch()),
                    stroke,
                );
            }
        } else {
            tab.currently_drawn_note = None;
        }
    }
}
