use cubedaw_lib::{Id, Range, Section, Track};
use egui::{pos2, vec2, Color32, Pos2, Rangef, Rect, Rounding};

use crate::app::Tab;

use super::track::TrackTab;

#[derive(Debug)]
pub struct PianoRollTab {
    id: Id<Tab>,

    track: Option<Id<Track>>,

    vertical_zoom: f32,
    horizontal_zoom: f32,

    last_mouse_position: (i64, i32),
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

            last_mouse_position: (0, 0),
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

        let track = ctx.state.tracks.get(track_id);

        let (_, resp) = ui.allocate_exact_size(
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

        let step = ((Range::UNITS_PER_BEAT as f32 / self.horizontal_zoom * 0.1).min(256.0) as u32)
            .next_power_of_two() as i64;

        for i in min_pos / step..=max_pos / step {
            let pos = i * step;
            // TODO make this not hardcoded
            const BEATS_PER_BAR: i64 = 4;
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

        for (_, section_id) in track.sections() {
            let section = ctx.state.sections.get(section_id);

            let section_stroke = egui::Stroke::new(2.0, SECTION_COLOR.gamma_multiply(0.5));

            let screen_range = Rangef::new(
                note_pos_to_screen_pos((section.start(), 0)).x,
                note_pos_to_screen_pos((section.end(), 0)).x,
            );

            painter.rect_filled(
                Rect::from_x_y_ranges(screen_range, screen_rect.y_range()),
                Rounding::ZERO,
                SECTION_COLOR.gamma_multiply(0.2),
            );
            painter.vline(screen_range.min, screen_rect.y_range(), section_stroke);
            painter.vline(screen_range.max, screen_rect.y_range(), section_stroke);
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
        for (_, section_id) in track.sections() {
            let section = ctx.state.sections.get(section_id);

            let screen_range = Rangef::new(
                note_pos_to_screen_pos((section.start(), 0)).x,
                note_pos_to_screen_pos((section.end(), 0)).x,
            );

            let header_rect =
                Rect::from_x_y_ranges(screen_range.expand(1.0), top_bar_rect.y_range());

            painter.rect_filled(
                header_rect,
                Rounding {
                    nw: 6.0,
                    ne: 6.0,
                    sw: 0.0,
                    se: 0.0,
                },
                SECTION_COLOR.gamma_multiply(0.5),
            );

            let padding = header_rect.height() * 0.5;
            painter.text(
                pos2(header_rect.left() + padding, header_rect.top() + padding),
                egui::Align2::LEFT_CENTER,
                &section.name,
                egui::FontId::proportional(12.0),
                ui.visuals().widgets.inactive.text_color(),
            );

            // let header_res = ui.allocate_rect(header_rect, egui::Sense::click_and_drag());
        }

        let track = ctx.state.tracks.get_mut(track_id);

        if let Some(mouse_pos) = resp.hover_pos() {
            let (pos, pitch) = screen_pos_to_note_pos(mouse_pos);
            self.last_mouse_position = (pos, pitch);

            if ui.input(|i| i.modifiers.ctrl) {
                let screen_pos = note_pos_to_screen_pos((pos, pitch));
                painter.vline(
                    screen_pos.x,
                    Rangef::new(screen_pos.y, screen_pos.y + self.vertical_zoom),
                    egui::Stroke::new(2.0, ui.visuals().text_color()),
                );

                ui.ctx().set_cursor_icon(egui::CursorIcon::None);
            }
        }

        resp.context_menu(|ui| {
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
