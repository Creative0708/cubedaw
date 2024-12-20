use core::f32;
use std::collections::VecDeque;

use anyhow::Result;
use cubedaw_lib::{Id, Range, Track};
use egui::{vec2, Widget};

use crate::{
    app::Tab, command::track::UiTrackSelect, state::ui::TrackUiState, widget::EditableLabel,
};

#[derive(Debug)]
pub struct TrackTab {
    id: Id<Tab>,

    // Vertical zoom. Each track height is multiplied by this
    vertical_zoom: f32,
    // Horizontal zoom. Each tick is this wide
    horizontal_zoom: f32,

    track_whose_name_is_being_edited: Option<(Id<Track>, String)>,
    track_whose_name_was_being_edited_last_frame: Option<Id<Track>>,
}

const SONG_PADDING: i64 = 2 * Range::UNITS_PER_BEAT as i64;

impl crate::Screen for TrackTab {
    fn create(_state: &cubedaw_lib::State, _ui_state: &crate::UiState) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            vertical_zoom: 1.0,
            horizontal_zoom: 0.125,

            track_whose_name_is_being_edited: None,
            track_whose_name_was_being_edited_last_frame: None,
        }
    }

    fn id(&self) -> cubedaw_lib::Id<crate::app::Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Tracks".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) -> Result<()> {
        let mut prepared = Prepared::new(ctx, ui, self);

        // egui::ScrollArea::vertical().show(ui, |ui| {
        prepared.update(ui);
        // });

        Ok(())
    }
}

struct Prepared<'a, 'b> {
    ctx: &'a mut crate::Context<'b>,
    tab: &'a mut TrackTab,

    /// If the user is dragging and released the drag, would the drag succeed?
    ///
    /// Basically, is the drag in range of the track headers.
    dragging_would_succeed: bool,
    track_list: Vec<TrackListEntry<'b>>,
}

#[derive(Debug)]
struct TrackListEntry<'a> {
    /// If this track weren't selected and being dragged, what would be its y position?
    /// This is used for calculating drag positions.
    actual_pos: f32,

    is_highlighted: bool,

    track_id: Id<Track>,
    track: &'a Track,
    track_ui: &'a TrackUiState,
    position: f32,
    height: f32,
    indentation: f32,
}

impl TrackListEntry<'_> {
    fn track_header(
        &self,
        tracker: &mut crate::context::UiStateTracker,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        should_highlight: bool,
        id_source: u32,
        tab: &mut TrackTab,
    ) {
        let visuals = if should_highlight {
            &ui.visuals().widgets.hovered
        } else {
            &ui.visuals().widgets.inactive
        };
        ui.painter()
            .rect(rect, 0.0, visuals.bg_fill, visuals.bg_stroke);
        ui.painter()
            .hline(rect.x_range(), rect.top(), visuals.fg_stroke);
        ui.painter()
            .hline(rect.x_range(), rect.bottom(), visuals.fg_stroke);

        self.track_header_inner(
            tracker,
            &mut ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(rect.shrink(4.0))
                    .id_salt(id_source),
            ),
            tab,
        );
    }
    fn track_header_inner(
        &self,
        tracker: &mut crate::context::UiStateTracker,
        ui: &mut egui::Ui,
        tab: &mut TrackTab,
    ) {
        let Self {
            track_id, track_ui, ..
        } = *self;

        let mut new_track_name = track_ui.name.clone();
        ui.add(EditableLabel::new(&mut new_track_name).id_salt(track_id));

        if new_track_name != track_ui.name {
            tracker.add(
                move |ui_state: &mut crate::UiState,
                      _ephemeral_state: &mut crate::EphemeralState,
                      _action: crate::command::UiActionType| {
                    core::mem::swap(
                        &mut new_track_name,
                        &mut ui_state.tracks.force_get_mut(track_id).name,
                    );
                },
            );
        }
    }
}

impl<'a, 'b> Prepared<'a, 'b> {
    fn new(ctx: &'a mut crate::Context<'b>, ui: &mut egui::Ui, tab: &'a mut TrackTab) -> Self {
        let mut track_list: Vec<TrackListEntry> = vec![];
        let mut current_y = ui.max_rect().top();
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
                    let track_ui = ctx.ui_state.tracks.force_get(track_id);
                    track_list.push(TrackListEntry {
                        actual_pos: current_y,

                        is_highlighted: false,

                        track_id,
                        track,
                        track_ui,
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

        let mut dragging_would_succeed = false;

        let drag = &ctx.ephemeral_state.drag;
        let width_of_track_header_panel = ui.max_rect().width();
        if drag.is_being_dragged(Id::new("tracks"))
            && let Some(egui::Vec2 {
                x: raw_movement_x,
                y: raw_movement_y,
            }) = drag.raw_movement()
            && (-width_of_track_header_panel..=width_of_track_header_panel)
                .contains(&raw_movement_x)
        {
            dragging_would_succeed = true;

            assert!(
                track_list.len() <= u32::MAX as usize,
                "there are more than u32::MAX tracks. wat"
            );

            let mut dragging_track_list = Vec::new();
            let mut dragging_tracks = VecDeque::new();
            let mut not_dragging_tracks = Vec::new();

            for track_entry in track_list.into_iter() {
                if track_entry.track_ui.selected {
                    dragging_tracks.push_back(track_entry);
                } else {
                    not_dragging_tracks.push(track_entry);
                }
            }

            let mut current_y = ui.max_rect().top();
            for track_entry in not_dragging_tracks.into_iter().map(Some).chain([None]) {
                while dragging_tracks.front().is_some_and(|front| {
                    track_entry.as_ref().map_or(true, |track_entry| {
                        front.position + raw_movement_y < current_y + track_entry.height * 0.5
                    })
                }) {
                    let front = dragging_tracks.pop_front().unwrap();
                    let position = {
                        let y = current_y;
                        current_y += front.height;
                        y
                    };
                    dragging_track_list.push(TrackListEntry { position, ..front });
                }

                if let Some(track_entry) = track_entry {
                    let position = {
                        let y = current_y;
                        current_y += track_entry.height;
                        y
                    };
                    dragging_track_list.push(TrackListEntry {
                        position,
                        ..track_entry
                    });
                }
            }

            track_list = dragging_track_list;
        }

        Self {
            ctx,
            tab,
            dragging_would_succeed,
            track_list,
        }
    }

    fn update(&mut self, ui: &mut egui::Ui) {
        let ctx = &mut *self.ctx;
        egui::SidePanel::left(egui::Id::new(self.tab.id)).show_inside(ui, |ui| {
            let result = ctx.ephemeral_state.drag.handle(
                Id::new("tracks"),
                |prepared: &mut crate::util::Prepared<'_, Id<Track>>| {
                    for track_entry in &mut self.track_list {
                        let rect = egui::Rect {
                            min: egui::pos2(
                                ui.max_rect().left() + track_entry.indentation,
                                track_entry.position,
                            ),
                            max: egui::pos2(
                                ui.max_rect().right(),
                                track_entry.position + track_entry.height,
                            ),
                        };
                        let response = ui.interact(
                            rect,
                            egui::Id::new((track_entry.track_id, "track_header")),
                            egui::Sense::click_and_drag(),
                        );
                        ui.advance_cursor_after_rect(rect);

                        prepared.process_interaction(
                            track_entry.track_id.cast(),
                            &response,
                            track_entry.track_id,
                            track_entry.track_ui.selected,
                        );

                        if response.double_clicked() {
                            // group tracks don't have sections (yet!) and selecting a group track is invalid
                            if track_entry.track.inner.is_section() {
                                ctx.tabs
                                    .get_or_create_tab::<super::pianoroll::PianoRollTab>(
                                        ctx.state,
                                        ctx.ui_state,
                                    )
                                    .select_track(Some(track_entry.track_id));
                            }
                            if let Some(patch_tab) = ctx.tabs.get_tab::<super::patch::PatchTab>() {
                                patch_tab.select_track(Some(track_entry.track_id));
                            }
                        }

                        // if not for the `!prepared.is_something_being_dragged()`,
                        // egui would think the rect at this position is hovered during dragging (which it's not)
                        // so this is a workaround.
                        track_entry.is_highlighted = !prepared.is_being_dragged()
                            && track_entry.track_ui.selected
                            || response.hovered();

                        ui.add_enabled_ui(
                            !(prepared.is_being_dragged()
                                && track_entry.track_ui.selected
                                && self.dragging_would_succeed),
                            |ui| {
                                track_entry.track_header(
                                    &mut ctx.tracker,
                                    ui,
                                    rect,
                                    track_entry.is_highlighted,
                                    0,
                                    self.tab,
                                );
                            },
                        );
                    }
                    if let Some(movement) = prepared.movement() {
                        // render the currently dragged tracks
                        let dragged_layer_id = egui::LayerId::new(
                            egui::Order::Foreground,
                            ui.layer_id().id.with("track drag"),
                        );
                        for track_entry in &self.track_list {
                            if !track_entry.track_ui.selected {
                                continue;
                            }
                            let untransformed_rect = egui::Rect {
                                min: egui::pos2(
                                    ui.max_rect().left() + track_entry.indentation,
                                    track_entry.actual_pos,
                                ),
                                max: egui::pos2(
                                    ui.max_rect().right(),
                                    track_entry.actual_pos + track_entry.height,
                                ),
                            };
                            let transformed_rect = untransformed_rect.translate(movement);

                            // if transformed_rect.any_nan() {
                            //     dbg!(track_entry.track_ui.selected, track_entry.actual_pos);
                            // }

                            ui.with_layer_id(dragged_layer_id, |ui| {
                                ui.set_clip_rect(ui.ctx().screen_rect());
                                track_entry.track_header(
                                    &mut ctx.tracker,
                                    ui,
                                    transformed_rect,
                                    true,
                                    1,
                                    self.tab,
                                );
                            });
                        }
                    }
                },
            );
            let viewport_interaction = ui.response();
            viewport_interaction.context_menu(|ui| {
                let mut b = ctx.ui_state.show_root_track;
                ui.checkbox(&mut b, "Show Master Track");
                if b != ctx.ui_state.show_root_track {
                    use crate::command::UiActionType;
                    ctx.tracker.add_weak(
                        move |ui_state: &mut crate::UiState,
                              _ephemeral_state: &mut crate::EphemeralState,
                              action: UiActionType| {
                            ui_state.show_root_track = match action {
                                UiActionType::Execute => b,
                                UiActionType::Rollback => !b,
                            };
                        },
                    );
                }
            });
            {
                let should_deselect_everything =
                    result.should_deselect_everything || viewport_interaction.clicked();
                let selection_changes = result.selection_changes;
                if should_deselect_everything {
                    for track_entry in &self.track_list {
                        if track_entry.track_ui.selected
                            && !matches!(selection_changes.get(&track_entry.track_id), Some(true))
                        {
                            ctx.tracker
                                .add(UiTrackSelect::new(track_entry.track_id, false));
                        }
                    }
                    for (&track_id, &selected) in selection_changes.iter() {
                        if selected
                            && !ctx
                                .ui_state
                                .tracks
                                .get(track_id)
                                .is_some_and(|n| n.selected)
                        {
                            ctx.tracker.add(UiTrackSelect::new(track_id, true));
                        }
                    }
                } else {
                    for (&track_id, &selected) in &selection_changes {
                        ctx.tracker.add(UiTrackSelect::new(track_id, selected));
                    }
                }
                if let Some(finished_drag_offset) = result.movement {
                    // for (&node_id, node_ui) in &track_ui.patch.nodes {
                    //     if node_ui.selected {
                    //         ctx.tracker.add(UiNodeMove::new(
                    //             node_id,
                    //             track_id,
                    //             finished_drag_offset,
                    //         ));
                    //     }
                    // }

                    // TODO
                }
            }
        });

        egui::CentralPanel::frame(Default::default(), Default::default()).show_inside(ui, |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink(egui::Vec2b::FALSE)
                .show_viewport(ui, |ui, viewport| {
                    let max_rect = ui.max_rect();
                    let top_left = max_rect.left_top();
                    let screen_rect = viewport.translate(top_left.to_vec2());

                    ui.painter().rect_filled(
                        screen_rect,
                        egui::Rounding::ZERO,
                        ui.visuals().extreme_bg_color,
                    );
                    // let rect = egui::Rect::from_x_y_ranges(
                    //     screen_rect,
                    //     // ui.max_rect().left()
                    //     //     ..=ui.max_rect().left()
                    //     //         + self.tab.horizontal_zoom
                    //     //             * (ctx.state.song_boundary.length() + 2 * SONG_PADDING)
                    //     //                 as f32,
                    //     // track_entry.position..=track_entry.position + track_entry.height,
                    // );

                    let response = ui.allocate_rect(screen_rect, egui::Sense::click_and_drag());

                    for track_entry in &self.track_list {
                        let highlighted = track_entry.is_highlighted;
                        let visuals = if track_entry.track_ui.selected
                            && ctx.ephemeral_state.drag.is_being_dragged(Id::new("tracks"))
                        {
                            ui.visuals().widgets.noninteractive
                        } else if highlighted {
                            ui.visuals().widgets.hovered
                        } else {
                            ui.visuals().widgets.inactive
                        };

                        let y_range = egui::Rangef::new(
                            track_entry.position,
                            track_entry.position + track_entry.height,
                        );

                        ui.painter().hline(
                            screen_rect.x_range(),
                            track_entry.position,
                            visuals.fg_stroke,
                        );
                        if highlighted {
                            ui.painter().rect_filled(
                                egui::Rect::from_x_y_ranges(screen_rect.x_range(), y_range),
                                0.0,
                                egui::Color32::from_gray(20).additive(),
                            );
                        }
                    }
                });
        });
    }
}
