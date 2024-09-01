use cubedaw_lib::{Id, Range, Track};
use egui::vec2;

use crate::{app::Tab, command::track::UiTrackRename, state::ui::TrackUiState};

#[derive(Debug)]
pub struct TrackTab {
    id: Id<Tab>,

    // Vertical zoom. Each track height is multiplied by this
    vertical_zoom: f32,
    // Horizontal zoom. Each tick is this wide
    horizontal_zoom: f32,

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

        match self.track_whose_name_is_being_edited {
            Some((edited_track_id, ref mut string)) if edited_track_id == track_id => {
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
            }
            _ => {
                let resp = ui.heading(&ui_state.name);
                if resp.double_clicked() {
                    self.track_whose_name_is_being_edited = Some((track_id, String::new()));
                }
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

            vertical_zoom: 1.0,
            horizontal_zoom: 0.125,

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
        let mut prepared = Prepared::new(ctx, ui, self);

        prepared.update(ui);
    }
}

struct Prepared<'a, 'b> {
    ctx: &'a mut crate::Context<'b>,
    tab: &'a mut TrackTab,

    track_list: Vec<TrackListEntry<'b>>,
}

#[derive(Debug)]
struct TrackListEntry<'a> {
    track: &'a Track,
    track_ui: &'a TrackUiState,
    position: f32,
    height: f32,
    indentation: f32,
}

impl<'a, 'b> Prepared<'a, 'b> {
    fn new(ctx: &'a mut crate::Context<'b>, ui: &mut egui::Ui, tab: &'a mut TrackTab) -> Self {
        let mut track_list: Vec<TrackListEntry> = vec![];
        let mut current_y = 0.0;
        if ctx.state.tracks.get(ctx.state.root_track).is_some() {
            // traverse the track list in order

            let mut track_stack: Vec<(Id<Track>, i32)> = vec![(
                ctx.state.root_track,
                if ctx.ui_state.show_root_track { 0 } else { -1 },
            )];

            while let Some((track_id, depth)) = track_stack.pop() {
                let track = ctx.state.tracks.force_get(track_id);

                // depth < 0 only happens when the track is a root track
                // and ctx.ui_state.show_root_track is false
                if depth >= 0 {
                    let height = 48.0; // TODO make configurable
                    track_list.push(TrackListEntry {
                        track,
                        track_ui: ctx.ui_state.tracks.force_get(track_id),
                        position: current_y,
                        height,
                        indentation: depth as f32 * 16.0,
                    });
                    current_y += height;
                }

                if matches!(track.inner, cubedaw_lib::TrackInner::Group(_)) {
                    for &child_id in &ctx.ui_state.tracks.force_get(track_id).track_list {
                        track_stack.push((child_id, depth + 1));
                    }
                }
            }
        }
        Self {
            ctx,
            tab,
            track_list,
        }
    }

    fn update(&mut self, ui: &mut egui::Ui) {
        let Self {
            ctx,
            tab,
            track_list,
        } = self;

        egui::ScrollArea::both()
            .auto_shrink(egui::Vec2b::FALSE)
            .show(ui, |ui| {
                // track headers
                egui::SidePanel::left(egui::Id::new((self.tab.id, "track_headers")))
                    .frame(Default::default())
                    .show_inside(ui, |ui| {
                        ui.interact_bg(egui::Sense::click()).context_menu(|ui| {
                            let mut b = ctx.ui_state.show_root_track;
                            ui.checkbox(&mut b, "Show Master Track");
                            if b != ctx.ui_state.show_root_track {
                                // TODO
                            }
                        });
                        for track_entry in &self.track_list {
                            let rect = egui::Rect::from_min_size(
                                ui.max_rect().left_top() + egui::vec2(0.0, track_entry.position),
                                egui::vec2(200.0, track_entry.height),
                            );
                            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                            ui.painter().rect(
                                rect,
                                0.0,
                                ui.visuals().window_fill,
                                ui.visuals().window_stroke(),
                            );

                            track_entry.track_header(&mut ui.child_ui(
                                rect.shrink(4.0),
                                *ui.layout(),
                                None,
                            ));
                        }
                    });
                egui::CentralPanel::frame(Default::default(), Default::default()).show_inside(
                    ui,
                    |ui| {
                        ui.painter()
                            .rect_filled(ui.max_rect(), 0.0, ui.visuals().extreme_bg_color);

                        for track_entry in &self.track_list {
                            let rect = egui::Rect::from_x_y_ranges(
                                ui.max_rect().left()
                                    ..=ui.max_rect().left()
                                        + self.tab.horizontal_zoom
                                            * (ctx.state.song_boundary.length() + 2 * SONG_PADDING)
                                                as f32,
                                track_entry.position..=track_entry.position + track_entry.height,
                            );
                            let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                        }
                    },
                );
            });
    }
}

impl TrackListEntry<'_> {
    fn track_header(&self, ui: &mut egui::Ui) {
        ui.heading(&self.track_ui.name);

        // TODO
    }
}
