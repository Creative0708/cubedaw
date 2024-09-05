use core::f32;
use std::collections::VecDeque;

use cubedaw_lib::{Id, Range, Track};
use egui::{vec2, Widget};

use crate::{app::Tab, command::track::UiTrackSelect, state::ui::TrackUiState};

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

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        let mut prepared = Prepared::new(ctx, ui, self);

        // egui::ScrollArea::vertical().show(ui, |ui| {
        prepared.update(ui);
        // });
    }
}

struct Prepared<'a, 'b> {
    ctx: &'a mut crate::Context<'b>,
    tab: &'a mut TrackTab,

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
            &mut ui.child_ui_with_id_source(rect.shrink(4.0), *ui.layout(), id_source, None),
            tab,
        );
    }
    fn track_header_inner(
        &self,
        tracker: &mut crate::context::UiStateTracker,
        ui: &mut egui::Ui,
        tab: &mut TrackTab,
    ) {
        match tab.track_whose_name_is_being_edited {
            Some((edited_track_id, ref mut string)) if edited_track_id == self.track_id => {
                let textedit_id = ui.auto_id_with("textedit");
                // gained_focus() doesn't work for some reason. TODO investigate
                let gained_focus = tab.track_whose_name_was_being_edited_last_frame.is_none();
                if gained_focus {
                    let mut state = egui::text_edit::TextEditState::default();
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(string.len()),
                        )));
                    state.store(ui.ctx(), textedit_id);
                }
                let resp = ui.add(egui::TextEdit::singleline(string).id(textedit_id));
                if gained_focus {
                    resp.request_focus();
                }
                if resp.lost_focus() {
                    if !ui.input(|i| i.key_pressed(egui::Key::Escape))
                        && &self.track_ui.name != string
                    {
                        let mut new_track_name = core::mem::take(string);
                        if !new_track_name.is_empty() {
                            let track_id = self.track_id;
                            tracker.add(move |ui_state: &mut crate::UiState, _ephemeral_state: &mut crate::EphemeralState, _action: crate::command::UiActionType| {
                                core::mem::swap(&mut new_track_name, &mut ui_state.tracks.force_get_mut(track_id).name);
                            });
                        }
                    }
                    tab.track_whose_name_is_being_edited = None;
                }

                tab.track_whose_name_was_being_edited_last_frame = tab
                    .track_whose_name_is_being_edited
                    .as_ref()
                    .map(|(id, _)| *id);
            }
            _ => {
                let label_resp =
                    egui::Label::new(egui::RichText::new(&self.track_ui.name).heading())
                        .sense(egui::Sense::hover())
                        .selectable(false)
                        .ui(ui);
                // expand label rect to edge of container to make renaming easier
                let resp = ui
                    .interact(
                        label_resp.rect.with_max_x(ui.max_rect().right()),
                        label_resp.id,
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::Text);
                if resp.double_clicked() {
                    tab.track_whose_name_is_being_edited =
                        Some((self.track_id, self.track_ui.name.clone()));
                }
            }
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
                    track_list.push(TrackListEntry {
                        actual_pos: current_y,

                        is_highlighted: false,

                        track_id,
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

        if let Some(raw_movement_y) = ctx.ephemeral_state.track_drag.raw_movement_y() {
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
            track_list,
        }
    }

    fn update(&mut self, ui: &mut egui::Ui) {
        let ctx = &mut *self.ctx;
        egui::SidePanel::left(egui::Id::new(self.tab.id)).show_inside(ui, |ui| {
            let result = ctx.ephemeral_state.track_drag.handle(
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
                            ctx.tabs
                                .get_or_create_tab::<super::pianoroll::PianoRollTab>(
                                    ctx.state,
                                    ctx.ui_state,
                                )
                                .select_track(Some(track_entry.track_id));
                        }

                        // if not for the `!prepared.is_something_being_dragged()`,
                        // egui would think the rect at this position is hovered during dragging (which it's not)
                        // so this is a workaround.
                        track_entry.is_highlighted = !prepared.is_something_being_dragged()
                            && (track_entry.track_ui.selected)
                            || response.hovered();

                        ui.add_enabled_ui(
                            !(prepared.is_something_being_dragged()
                                && track_entry.track_ui.selected),
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

                            ui.scope(|ui| {
                                ui.painter().set_layer_id(egui::LayerId {
                                    order: egui::Order::Tooltip,
                                    id: egui::Id::
                                })
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
            let viewport_interaction = ui.interact_bg(egui::Sense::click());
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
                            && ctx.ephemeral_state.track_drag.is_something_being_dragged()
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
