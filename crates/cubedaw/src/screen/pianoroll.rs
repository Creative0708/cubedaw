use cubedaw_lib::{
    math::frange_viewport,
    track::{Note, Section, Track, TrackData},
    Id, IdCorrespondenceMap as _, Range,
};
use egui::{pos2, vec2, Color32, Pos2, Rect, Rounding, ScrollArea, Sense, TopBottomPanel};

use super::{Screen, TrackScreen};

pub struct PianoRollScreen {
    id: Id<PianoRollScreen>,

    // width of 1 song unit
    horizontal_zoom: f32,
    // height of 1 note
    vertical_zoom: f32,

    note_being_drawn: Option<Note>,
    // Option<(selected track, Option<selected section>)>
    selected: Option<(Id<Track>, Option<Id<Section>>)>,
}

impl PianoRollScreen {
    pub const NUM_NOTES_VERICALLY: u32 = 128;

    pub fn new(
        id: Id<PianoRollScreen>,
        selected: Option<(Id<Track>, Option<Id<Section>>)>,
    ) -> Self {
        Self {
            id,
            horizontal_zoom: 0.5,
            vertical_zoom: 24.0,
            note_being_drawn: None,
            selected,
        }
    }

    fn update_inner(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, viewport: Rect) {
        let Some((track_id, selection_ref)) = self.selected else {
            return;
        };

        let (res, painter) = ui.allocate_painter(
            vec2(
                self.horizontal_zoom * ctx.state.song_range.length() as f32,
                self.vertical_zoom * Self::NUM_NOTES_VERICALLY as f32,
            ),
            Sense::click_and_drag(),
        );

        let translation = res.rect.min.to_vec2();

        let ((vmin_pitch, vmin_pos), (vmax_pitch, vmax_pos)) = (
            self.get_song_pos(ui, ctx, viewport, viewport.min),
            self.get_song_pos(ui, ctx, viewport, viewport.max),
        );

        painter.rect_filled(
            viewport.translate(translation),
            Rounding::ZERO,
            ui.style().visuals.extreme_bg_color,
        );

        // ui.painter().rect_filled(
        //     Rect::from_x_y_ranges(viewport.x_range(), 0.0..=5.0),
        //     Rounding::ZERO,
        //     Color32::DEBUG_COLOR,
        // )

        for (i, pos) in frange_viewport(self.vertical_zoom, viewport.top(), viewport.bottom()) {
            let note_id = Self::NUM_NOTES_VERICALLY - i - 1;

            const IS_BLACK_NOTE: &[bool] = &[
                false, true, false, true, false, false, true, false, true, false, true, false,
            ];

            if IS_BLACK_NOTE[note_id as usize % IS_BLACK_NOTE.len()] {
                painter.rect_filled(
                    Rect::from_x_y_ranges(viewport.x_range(), pos..=pos + self.vertical_zoom)
                        .translate(translation),
                    Rounding::ZERO,
                    ui.style().visuals.faint_bg_color,
                );
            }
        }

        let track = ctx.state.track_map.id_get_mut(track_id);
        let TrackData::SynthesizerTrack(track_data) = &mut track.track_data else {
            todo!()
        };

        if self.note_being_drawn.is_some() {
            if let Some(pointer_pos) = ui.input(|i| {
                if i.pointer.primary_down() {
                    i.pointer.latest_pos()
                } else {
                    None
                }
            }) {
                if let Some((pitch, pos)) = self.try_get_song_pos(ui, ctx, viewport, pointer_pos) {
                    let note_being_drawn = self.note_being_drawn.as_mut().unwrap();
                    *note_being_drawn.end_mut() = pos.max(note_being_drawn.start());
                    note_being_drawn.pitch = pitch;
                }
            } else {
                let note = self.note_being_drawn.take().unwrap();
                track_data
                    .get_or_create_section_at(&mut ctx.state.section_map, note.start())
                    .insert_note(note);
            }
        } else if res.hovered() && ui.input(|i| i.modifiers.ctrl) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            if let Some(pointer_pos) = ui.input(|i| i.pointer.latest_pos()) {
                if let Some((pitch, pos)) = self.try_get_song_pos(ui, ctx, viewport, pointer_pos) {
                    let screen_pos = self.get_screen_pos(ui, pitch, pos);
                    ui.painter().line_segment(
                        [screen_pos, screen_pos + vec2(0.0, self.vertical_zoom)],
                        ui.style().visuals.widgets.open.fg_stroke,
                    );
                    if ui.input(|i| i.pointer.primary_down()) && self.note_being_drawn.is_none() {
                        self.note_being_drawn =
                            Some(Note::from_range_pitch(Range::new_at(pos), pitch))
                    }
                }
            }
        }

        let track = ctx.state.track_map.id_get_mut(track_id);
        let TrackData::SynthesizerTrack(track_data) = &mut track.track_data else {
            todo!()
        };

        let mut count = 0;
        for id in track_data.section_ids() {
            let section = ctx.state.section_map.id_get_mut(id);
            if section.start() >= vmax_pos {
                break;
            } else if section.end() < vmin_pos {
                continue;
            }
            log::info!("funny {:?} {:?}", id, section as *const _);

            for note in section.notes_mut() {
                count += 1;
                self.handle_note(ui, viewport, note);
            }
        }
        log::info!("handled {count} notes, current {:?}", self.note_being_drawn);
    }

    fn handle_note(&self, ui: &mut egui::Ui, viewport: Rect, note: &mut Note) {
        const NOTE_COLOR: Color32 = Color32::DEBUG_COLOR;
        log::info!("handling {note:?}");

        let note_rect = Rect::from_min_max(
            self.get_screen_pos(ui, note.pitch + 1, note.start()),
            self.get_screen_pos(ui, note.pitch, note.end()),
        );
        if !note_rect.intersects(viewport) {
            return;
        }

        ui.painter()
            .rect_filled(note_rect, Rounding::ZERO, NOTE_COLOR);
    }

    fn get_relative_pos(&self, pitch: u32, pos: i64) -> Pos2 {
        pos2(
            pos as f32 * self.horizontal_zoom,
            (Self::NUM_NOTES_VERICALLY - pitch - 1) as f32 * self.vertical_zoom,
        )
    }
    fn get_screen_pos(&self, ui: &mut egui::Ui, pitch: u32, pos: i64) -> Pos2 {
        return self.get_relative_pos(pitch, pos) + ui.max_rect().min.to_vec2();
    }

    fn try_get_screen_pos(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        viewport: Rect,
        pitch: u32,
        pos: i64,
    ) -> Option<Pos2> {
        let relative_pos = self.get_relative_pos(pitch, pos);

        let mut expanded_viewport = viewport;
        *expanded_viewport.top_mut() -= self.vertical_zoom;

        if !expanded_viewport.contains(relative_pos) {
            return None;
        }

        return Some(relative_pos + ui.max_rect().min.to_vec2());
    }

    fn try_get_song_pos(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        viewport: Rect,
        screen_pos: Pos2,
    ) -> Option<(u32, i64)> {
        if !viewport
            .translate(ui.max_rect().min.to_vec2())
            .contains(screen_pos)
        {
            return None;
        }

        Some(self.get_song_pos(ui, ctx, viewport, screen_pos))
    }

    fn get_song_pos(
        &self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        viewport: Rect,
        screen_pos: Pos2,
    ) -> (u32, i64) {
        let relative_pos = screen_pos - ui.max_rect().min.to_vec2();

        (
            Self::NUM_NOTES_VERICALLY - (relative_pos.y / self.vertical_zoom) as u32 - 1,
            (relative_pos.x / self.horizontal_zoom) as i64 + ctx.state.song_range.start,
        )
    }

    pub fn select(&mut self, selected: Option<(Id<Track>, Option<Id<Section>>)>) {
        // if self.selected_track == track_id {
        //     return;
        // }
        self.selected = selected;
    }
}

impl Screen for PianoRollScreen {
    fn id(&self) -> Id<()> {
        self.id.transmute()
    }

    fn title(&self) -> egui::WidgetText {
        "Piano Roll".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        TopBottomPanel::top(self.id.with("top")).show_inside(ui, |ui| {
            ui.label("text here");
        });
        ScrollArea::both()
            .auto_shrink([false, false])
            .drag_to_scroll(false)
            .show_viewport(ui, |ui, viewport| {
                self.update_inner(ctx, ui, viewport);
            });
    }

    fn create(ctx: &mut crate::Context) -> Self {
        Self::new(
            Id::arbitrary(),
            ctx.tabs.get_tab::<TrackScreen>().and_then(|scr| {
                scr.get_single_selected_track_and_section(&mut ctx.state.track_map)
            }),
        )
    }
}
