use cubedaw_lib::{Id, Range, Section, Track};
use egui::{pos2, vec2, Pos2, Rangef, Rect, Rounding};

use crate::app::Tab;

use super::track::TrackTab;

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track: Option<Id<Track>>,

    vertical_zoom: f32,
    horizontal_zoom: f32,
}

const SONG_PADDING: i64 = 8 * Range::UNITS_PER_BEAT;
const MIN_NOTE_SHOWN: i32 = -39;
const MAX_NOTE_SHOWN: i32 = 47;

impl crate::Screen for PianoRollTab {
    fn create(ctx: &mut crate::Context) -> Self {
        Self {
            id: Id::arbitrary(),

            track: ctx
                .tabs
                .get_tab::<TrackTab>()
                .map(|t| t.get_single_selected_track()),

            vertical_zoom: 16.0,
            horizontal_zoom: 1.0,
        }
    }

    fn id(&self) -> Id<Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Piano Roll".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        let mut prepared = self.begin(ctx);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().show_viewport(ui, |ui, viewport| {
                if prepared.track.is_some() {
                    prepared.pianoroll(ui, viewport);
                } else {
                    prepared.pianoroll_empty(ui);
                }
            })
        });
    }
}

impl PianoRollTab {
    fn begin<'a, 'b>(&'a mut self, ctx: &'a mut crate::Context<'b>) -> Prepared<'a, 'b> {
        Prepared {
            ctx,

            track: self.track,

            vertical_zoom: self.vertical_zoom,
            horizontal_zoom: self.horizontal_zoom,
        }
    }
}

struct Prepared<'a, 'b> {
    ctx: &'a mut crate::Context<'b>,

    track: Option<Id<Track>>,

    vertical_zoom: f32,
    horizontal_zoom: f32,
}

impl<'a, 'b> Prepared<'a, 'b> {
    fn pianoroll(&mut self, ui: &mut egui::Ui, viewport: Rect) {
        let Some(track) = self.track else {
            unreachable!()
        };

        let (_, resp) = ui.allocate_exact_size(
            vec2(
                (self.ctx.state.song_boundary.length() + SONG_PADDING * 2) as f32
                    * self.horizontal_zoom,
                (MAX_NOTE_SHOWN - MIN_NOTE_SHOWN) as f32 * self.vertical_zoom,
            ),
            egui::Sense::click_and_drag(),
        );

        let max_rect = ui.max_rect();
        let top_left = max_rect.left_top().to_vec2();
        let painter = ui.painter_at(viewport.translate(top_left));

        painter.rect_filled(max_rect, Rounding::ZERO, ui.visuals().extreme_bg_color);

        let screen_pos_to_note_pos = |screen_pos: Pos2| -> (i64, i32) {
            let ui_pos = screen_pos - top_left;
            (
                (ui_pos.x / self.horizontal_zoom) as i64 + self.ctx.state.song_boundary.start
                    - SONG_PADDING,
                (ui_pos.y / self.vertical_zoom) as i32 + MIN_NOTE_SHOWN,
            )
        };
        let note_pos_to_screen_pos = |(pos, pitch): (i64, i32)| -> Pos2 {
            let ui_pos = pos2(
                (pos - self.ctx.state.song_boundary.start + SONG_PADDING) as f32
                    * self.horizontal_zoom,
                (pitch - MIN_NOTE_SHOWN) as f32 * self.vertical_zoom,
            );
            ui_pos + top_left
        };

        for row in (viewport.top() / self.vertical_zoom) as i32.. {
            let row_pos = row as f32 * self.vertical_zoom;
            if row_pos > viewport.bottom() {
                break;
            }
            if row % 2 == 0 {
                painter.rect_filled(
                    Rect::from_x_y_ranges(
                        viewport.x_range(),
                        Rangef::new(row_pos, row_pos + self.vertical_zoom),
                    )
                    .translate(top_left),
                    Rounding::ZERO,
                    ui.visuals().faint_bg_color.gamma_multiply(2.0),
                );
            }
        }

        let track = self.ctx.state.tracks.get_mut(track);

        if let Some(mouse_pos) = resp.hover_pos() {
            let (pos, pitch) = screen_pos_to_note_pos(mouse_pos);

            if ui.input(|i| i.modifiers.ctrl) {
                let screen_pos = note_pos_to_screen_pos((pos, pitch));
                painter.vline(
                    screen_pos.x,
                    Rangef::new(screen_pos.y, screen_pos.y + self.vertical_zoom),
                    egui::Stroke::new(2.0, ui.visuals().text_color()),
                );

                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            }
            resp.context_menu(|ui| {
                if ui.button("Add section").clicked() {
                    track.add_section(
                        &mut self.ctx.state.sections,
                        Section::empty(Range::surrounding_pos(pos)),
                    );
                }
            });
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
