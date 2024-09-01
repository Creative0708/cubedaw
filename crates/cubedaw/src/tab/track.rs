use cubedaw_lib::{Id, Range, Track};
use egui::vec2;

use crate::{app::Tab, command::track::UiTrackRename};

#[derive(Debug)]
pub struct TrackTab {
    id: Id<Tab>,

    // Vertical zoom. Each track height is multiplied by this
    units_per_track_unit: f32,
    // Horizontal zoom. Each tick is this wide
    units_per_tick: f32,

    // TODO rename this
    track_whose_name_is_being_edited: Option<(Id<Track>, String)>,
}

const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT as i64;

impl TrackTab {
    fn track_header(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, track_id: Id<Track>) {
        let ui_state = ctx
            .ui_state
            .tracks
            .get(track_id)
            .expect("existing track has no ui state");

        // TODO dear god
        if if let Some((edited_track_id, ref mut string)) = self.track_whose_name_is_being_edited {
            if edited_track_id == track_id {
                let resp = ui.add(egui::TextEdit::singleline(string));
                if resp.lost_focus() {
                    let new_track_name = core::mem::take(string);
                    if !new_track_name.is_empty() {
                        ctx.tracker
                            .add(UiTrackRename::new(track_id, new_track_name));
                    }
                    self.track_whose_name_is_being_edited = None;
                } else {
                    resp.request_focus();
                }
                false
            } else {
                true
            }
        } else {
            true
        } {
            let resp = ui.heading(&ui_state.name);
            if resp.double_clicked() {
                self.track_whose_name_is_being_edited = Some((track_id, String::new()));
            }
        }
    }
}

impl crate::Screen for TrackTab {
    fn create(_ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            units_per_track_unit: 1.0,
            units_per_tick: 0.125,

            track_whose_name_is_being_edited: None,
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
                    self.track_header(ctx, &mut ui.child_ui(rect, *ui.layout(), None), track_id);
                }
            });
    }
}
