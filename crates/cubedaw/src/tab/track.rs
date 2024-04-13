use cubedaw_lib::{Id, Range, Track};
use egui::vec2;

use crate::app::Tab;

#[derive(Debug)]
pub struct TrackTab {
    id: Id<Tab>,

    // Vertical zoom. Each track height is multiplied by this
    units_per_track_unit: f32,
    // Horizontal zoom. Each tick is this wide
    units_per_tick: f32,
}

const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT;

impl crate::Screen for TrackTab {
    fn create(_ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            units_per_track_unit: 1.0,
            units_per_tick: 0.125,
        }
    }

    fn id(&self) -> cubedaw_lib::Id<crate::app::Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Tracks".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        egui::ScrollArea::both()
            .auto_shrink(egui::Vec2b::FALSE)
            .show(ui, |ui| {
                // TODO don't hardcode this
                const TRACK_HEIGHT: f32 = 64.0;

                let tracks = ctx.ui_state.track_list.clone();
                for track_id in tracks {
                    let (rect, response) = ui.allocate_exact_size(
                        vec2(
                            self.units_per_tick
                                * (ctx.state.song_boundary.length() + 2 * SONG_PADDING) as f32,
                            self.units_per_track_unit * TRACK_HEIGHT,
                        ),
                        egui::Sense::click_and_drag(),
                    );
                    track_header(ctx, &mut ui.child_ui(rect, *ui.layout()), track_id);
                }
            });

        fn track_header(ctx: &mut crate::Context, ui: &mut egui::Ui, track_id: Id<Track>) {
            // let track = ctx.state.tracks.get_mut(track_id);
            let ui_data = ctx.ui_state.tracks.get_mut(track_id);

            if ui_data.is_editing_name {
                let resp = ui.add(egui::TextEdit::singleline(&mut ui_data.name));
                if resp.lost_focus() {
                    ui_data.is_editing_name = false;
                } else {
                    resp.request_focus();
                }
            } else {
                let resp = ui.heading(&ui_data.name);
                if resp.double_clicked() {
                    ui_data.is_editing_name = true;
                }
            }
        }
    }
}
