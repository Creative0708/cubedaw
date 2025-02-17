use cubedaw_lib::{PreciseSongPos, Range};
use egui::{CornerRadius, NumExt, Pos2, Rangef, Rect, Response, Vec2};

// TODO make these not hardcoded
const BEATS_PER_BAR: i64 = 4;
const TOP_BAR_HEIGHT: f32 = 18.0;

/// Shared functionality for the track view and the piano roll.
#[derive(Debug)]
pub struct SongViewer {
    // Horizontal zoom. Each tick is this wide
    pub units_per_tick: f32,
}

impl SongViewer {
    pub fn new() -> Self {
        Self {
            units_per_tick: 0.5,
        }
    }

    pub fn ui<F, R>(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, f: F) -> R
    where
        F: FnOnce(&mut crate::Context, &mut egui::Ui, &SongViewerPrepared) -> R,
    {
        egui::ScrollArea::both()
            .show_viewport(ui, |ui, viewport| {
                let prepared = SongViewerPrepared::new(self, ctx, ui, viewport);

                f(ctx, ui, &prepared)
            })
            .inner
    }

    pub fn top_bar_height(&self) -> f32 {
        // TODO not hardcode this (so people can configure it, etc.)
        TOP_BAR_HEIGHT
    }

    pub fn anchor(&self, ui: &egui::Ui) -> Pos2 {
        let mut anchor = ui.max_rect().left_top();
        anchor.y += self.top_bar_height();
        anchor
    }
}

impl Default for SongViewer {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SongViewerPrepared<'a> {
    pub max_rect: Rect,
    pub screen_rect: Rect,
    pub units_per_tick: f32,
    pub song_boundary: Range,

    pub song_view_range: Range,

    pub top_bar_rect: Rect,

    vbar_step: i64,

    // i know damn well a reference is gonna be needed at some point so add the lifetime specifier now
    _marker: core::marker::PhantomData<&'a ()>,
}

const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT as i64;

impl<'a> SongViewerPrepared<'a> {
    fn new(
        parent: &SongViewer,
        ctx: &mut crate::Context,
        ui: &mut egui::Ui,
        viewport: Rect,
    ) -> Self {
        let max_rect = ui.max_rect();
        // total screen rect, including the top bar
        let actual_screen_rect = viewport.translate(max_rect.left_top().to_vec2());
        // only the content rect
        let screen_rect = actual_screen_rect.with_min_y(actual_screen_rect.top() + TOP_BAR_HEIGHT);
        let units_per_tick = parent.units_per_tick;

        let mut this = Self {
            // we'll allocate some extra space of height TOP_BAR_HEIGHT in self.ui_background() so offset the rect
            max_rect: max_rect.with_min_y(max_rect.top() + TOP_BAR_HEIGHT),
            screen_rect: actual_screen_rect,
            units_per_tick,
            song_boundary: ctx.state.song_boundary,

            song_view_range: Default::default(),

            top_bar_rect: Rect::from_x_y_ranges(
                actual_screen_rect.x_range(),
                Rangef::new(actual_screen_rect.top(), screen_rect.top()),
            ),

            // the distance between rendered vertical lines. stays roughly the same value as the screen is horizontally zoomed.
            vbar_step: ((Range::UNITS_PER_BEAT as f32 / units_per_tick * 0.1).min(256.0) as u32)
                .next_power_of_two() as i64,

            _marker: core::marker::PhantomData,
        };

        this.song_view_range = Range::new(
            this.screen_x_to_song_x(actual_screen_rect.left()),
            this.screen_x_to_song_x(actual_screen_rect.right()),
        );

        this
    }

    #[doc(alias = "top_left")]
    pub fn anchor(&self) -> Pos2 {
        self.max_rect.left_top()
    }

    // TODO: all of these are hilariously imprecise. Fix later:tm:
    pub fn screen_x_to_song_x(&self, pos: f32) -> i64 {
        ((pos - self.anchor().x) / self.units_per_tick) as i64 + self.song_boundary.start
            - SONG_PADDING
    }
    pub fn song_x_to_screen_x(&self, pos: i64) -> f32 {
        (pos - (self.song_boundary.start - SONG_PADDING)) as f32 * self.units_per_tick
            + self.anchor().x
    }
    pub fn precise_x_to_screen_x(&self, pos: PreciseSongPos) -> f32 {
        /// `2.0 ** -64.0`
        const X: f32 = 5.421011e-20;
        self.song_x_to_screen_x(pos.song_pos) + pos.fraction as f32 * X * self.units_per_tick
    }
    pub fn song_range_to_screen_range(&self, range: Range) -> Rangef {
        Rangef::new(
            self.song_x_to_screen_x(range.start),
            self.song_x_to_screen_x(range.end),
        )
    }
    pub fn screen_range_to_song_range(&self, range: Rangef) -> Range {
        Range::new(
            self.screen_x_to_song_x(range.min),
            self.screen_x_to_song_x(range.max),
        )
    }

    /// Like `self.screen_x_to_note_x()`, but snaps to the nearest vertical line.
    pub fn input_screen_x_to_song_x(&self, pos: f32) -> i64 {
        let note_x = self.screen_x_to_song_x(pos);

        (note_x + self.vbar_step / 2).div_floor(self.vbar_step) * self.vbar_step
    }

    // ui functions
    pub fn ui_background(&self, ctx: &crate::Context, ui: &mut egui::Ui, height: f32) -> Response {
        let height = (TOP_BAR_HEIGHT + height).at_least(self.screen_rect.height());

        // Background rectangle
        ui.painter().rect_filled(
            self.screen_rect,
            CornerRadius::ZERO,
            ui.visuals().extreme_bg_color,
        );

        let bg_response = ui.allocate_rect(
            Rect::from_min_size(
                self.anchor(),
                Vec2::new(
                    (ctx.state.song_boundary.length() + SONG_PADDING * 2) as f32
                        * self.units_per_tick,
                    height,
                ),
            ),
            egui::Sense::click_and_drag(),
        );

        // Vertical bar/beat/whatever indicators

        for pos in self.song_view_range.iter_snap_to(self.vbar_step) {
            let stroke =
                if pos % (BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64) == 0 {
                    ui.visuals().widgets.hovered.bg_stroke
                } else {
                    const NUM_DIVISIONS_THING: u32 = 4;
                    let division = (pos.trailing_zeros() - self.vbar_step.trailing_zeros())
                        .min(NUM_DIVISIONS_THING);
                    egui::Stroke::new(
                        1.0,
                        ui.visuals().widgets.hovered.bg_stroke.color.gamma_multiply(
                            (division as f32 / NUM_DIVISIONS_THING as f32).powf(0.3),
                        ),
                    )
                };
            ui.painter().vline(
                self.song_x_to_screen_x(pos as _),
                self.screen_rect.y_range(),
                stroke,
            );
        }

        bg_response
    }
    pub fn ui_top_bar(&self, ctx: &mut crate::Context, ui: &mut egui::Ui) -> Response {
        let top_bar_rect = self.top_bar_rect;

        let top_bar_interaction = ui.allocate_rect(top_bar_rect, egui::Sense::click_and_drag());

        // background
        {
            ui.painter().rect_filled(
                top_bar_rect,
                CornerRadius::ZERO,
                ui.visuals().extreme_bg_color,
            );
            ui.painter().hline(
                top_bar_rect.x_range(),
                top_bar_rect.bottom(),
                ui.visuals().window_stroke,
            );
        }

        // Bar indicators
        const UNITS_PER_BAR: i64 = BEATS_PER_BAR * Range::UNITS_PER_BEAT as i64;
        for bar in self.song_view_range.multiples_within_range(UNITS_PER_BAR) {
            let pos = bar * UNITS_PER_BAR;

            ui.painter().text(
                Pos2::new(
                    self.song_x_to_screen_x(pos as _),
                    top_bar_rect.y_range().center(),
                ),
                egui::Align2::CENTER_CENTER,
                bar,
                egui::FontId::proportional(12.0),
                ui.visuals().widgets.hovered.text_color(),
            );
        }

        // if the user clicks on the top bar, the playhead should be set to the clicked position
        if top_bar_interaction.contains_pointer()
            && let Some(pointer_pos) = ui.input(|i| {
                if i.pointer.primary_clicked() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            })
        {
            ctx.tracker
                .add_weak(crate::command::misc::UiSetPlayhead::new(
                    self.input_screen_x_to_song_x(pointer_pos.x) as _,
                ));
        }

        top_bar_interaction
    }
    pub fn ui_playhead(&self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        let screen_rect = self.screen_rect;

        let playhead_screen_x = self.precise_x_to_screen_x(ctx.playhead_pos());
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
            let playhead_top_pos = Pos2::new(playhead_screen_x, screen_rect.top());

            // the triangle!!!
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(playhead_top_pos.x + 5.0, playhead_top_pos.y),
                    Pos2::new(playhead_top_pos.x, playhead_top_pos.y + 7.0),
                    Pos2::new(playhead_top_pos.x - 5.0, playhead_top_pos.y),
                ],
                playhead_stroke.color,
                egui::Stroke::NONE,
            ));
        }
    }
}
