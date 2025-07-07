use core::f32;
use std::collections::VecDeque;

use anyhow::Result;
use cubedaw_lib::{Id, IdMap, Range, Track};
use cubedaw_worker::command::ActionDirection;
use egui::{Color32, CursorIcon, Pos2, Rect, Sense, Stroke, StrokeKind, UiBuilder};

use crate::{
    app::Tab,
    state::ui::TrackUiState,
    util::Select,
    widget::{EditableLabel, SongViewer, SongViewerPrepared},
};

#[derive(Debug)]
pub struct TrackTab {
    id: Id<Tab>,

    // Vertical zoom. Each track height is multiplied by this
    // TODO implement
    vertical_zoom: f32,

    song_viewer: SongViewer,
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

            song_viewer: SongViewer {
                units_per_tick: 1.0 / 16.0,
            },
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
        egui::SidePanel::left(egui::Id::new(self.id)).show_inside(ui, |ui| {
            prepared.ui_left_sidebar(ctx, ui);
        });
        egui::CentralPanel::default()
            .frame(Default::default())
            .show_inside(ui, |ui| {
                self.song_viewer.ui(ctx, ui, |ctx, ui, view| {
                    prepared.central_panel(ui, ctx, view);
                })
            });

        Ok(())
    }
}

struct Prepared<'ctx> {
    tab_has_focus: bool,

    /// If the user is dragging and released the drag, would the drag succeed?
    ///
    /// Basically, is the drag in range of the track headers.
    dragging_would_succeed: bool,
    track_list: TrackList<'ctx>,
}

#[derive(Debug)]
struct TrackList<'ctx> {
    list: Vec<TrackListEntry<'ctx>>,
    top: f32,
    bottom: f32,
}

impl<'ctx> TrackList<'ctx> {
    fn new(list: Vec<TrackListEntry<'ctx>>, bottom: f32) -> Self {
        let top = match list.first() {
            Some(first) => first.actual_pos,
            None => bottom,
        };
        Self { list, top, bottom }
    }

    fn entry_at_y(&self, y: f32) -> Option<(u32, &TrackListEntry<'ctx>)> {
        let last = self.list.last()?;
        if !(self.top..last.actual_bottom()).contains(&y) {
            return None;
        }

        // the index of the first entry whose top is lower on the screen than the y (it doesn't contain the y)
        let index_of_first_lower = self.list.partition_point(|entry| y > entry.actual_pos);
        let index_of_entry = index_of_first_lower.checked_sub(1)?;
        Some((
            index_of_entry.try_into().unwrap(),
            &self.list[index_of_entry],
        ))
    }

    fn total_height(&self) -> f32 {
        self.bottom - self.top
    }

    fn len(&self) -> usize {
        self.list.len()
    }
}

#[derive(Debug)]
struct TrackListEntry<'a> {
    /// If this track weren't selected and being dragged, what would be its y position?
    /// This is used for calculating drag positions.
    actual_pos: f32,

    is_highlighted: bool,
    /// Is this track or any of its parents selected?
    would_be_dragged: bool,

    track_id: Id<Track>,
    track: &'a Track,
    track_ui: &'a TrackUiState,
    position: f32,
    height: f32,
    indentation: f32,
}
impl<'a> TrackListEntry<'a> {
    pub fn actual_bottom(&self) -> f32 {
        self.actual_pos + self.height
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Track2DPos {
    pub time: i64,
    pub idx: i32,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct Track2DOffset {
    pub time: i64,
    pub idx: i32,
}
impl std::ops::Sub for Track2DPos {
    type Output = Track2DOffset;
    fn sub(self, rhs: Self) -> Self::Output {
        Track2DOffset {
            time: self.time - rhs.time,
            idx: self.idx - rhs.idx,
        }
    }
}

impl TrackListEntry<'_> {
    fn track_header(
        &self,
        tracker: &mut crate::context::UiStateTracker,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        should_highlight: bool,
        id_source: u32,
    ) {
        let visuals = if should_highlight {
            &ui.visuals().widgets.hovered
        } else {
            &ui.visuals().widgets.inactive
        };
        ui.painter().rect_filled(rect, 0.0, visuals.bg_fill);
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
        );
    }
    fn track_header_inner(&self, tracker: &mut crate::context::UiStateTracker, ui: &mut egui::Ui) {
        let Self {
            track_id, track_ui, ..
        } = *self;

        let mut new_track_name = track_ui.name.clone();
        ui.add(EditableLabel::new(&mut new_track_name).id_salt(track_id));

        if new_track_name != track_ui.name {
            tracker.add(
                move |ui_state: &mut crate::UiState,
                      _ephemeral_state: &mut crate::EphemeralState,
                      _action: ActionDirection| {
                    core::mem::swap(
                        &mut new_track_name,
                        &mut ui_state.tracks.force_get_mut(track_id).name,
                    );
                },
            );
        }
    }
}

const DEFAULT_TRACK_HEIGHT: f32 = 48.0;

impl<'ctx> Prepared<'ctx> {
    fn new(ctx: &mut crate::Context<'ctx>, ui: &mut egui::Ui, tab: &mut TrackTab) -> Self {
        let mut track_entries: Vec<TrackListEntry> = vec![];
        let mut current_y = tab.song_viewer.anchor(ui).y;
        if ctx.state.tracks.get(ctx.state.root_track).is_some() {
            // traverse the track list in order

            let mut track_stack: Vec<(Id<Track>, i32, bool)> = vec![(
                ctx.state.root_track,
                if ctx.ui_state.show_root_track { 0 } else { -1 },
                false,
            )];

            while let Some((track_id, depth, parent_selected)) = track_stack.pop() {
                let track = ctx.state.tracks.force_get(track_id);

                let mut is_this_track_or_any_of_its_parents_selected = false;

                // depth < 0 only happens when the track is a root track and ctx.ui_state.show_root_track is false
                if depth >= 0 {
                    let track_ui = ctx.ui_state.tracks.force_get(track_id);
                    is_this_track_or_any_of_its_parents_selected =
                        parent_selected || track_ui.select.is();

                    let height = DEFAULT_TRACK_HEIGHT; // TODO make configurable
                    track_entries.push(TrackListEntry {
                        actual_pos: current_y,

                        is_highlighted: false,
                        would_be_dragged: is_this_track_or_any_of_its_parents_selected,

                        track_id,
                        track,
                        track_ui,
                        position: current_y,
                        height,
                        indentation: depth as f32 * 16.0,
                    });
                    current_y += height;
                }

                for &child_id in &ctx.ui_state.tracks.force_get(track_id).track_list {
                    track_stack.push((
                        child_id,
                        depth + 1,
                        is_this_track_or_any_of_its_parents_selected,
                    ));
                }
            }
        }
        let mut track_list = TrackList::new(track_entries, current_y);

        let mut dragging_would_succeed = false;

        let width_of_track_header_panel = ui.max_rect().width();
        if let Some(egui::Vec2 {
            x: raw_movement_x,
            y: raw_movement_y,
        }) = ctx.ephemeral_state.track_drag.raw_movement()
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

            for track_entry in track_list.list.into_iter() {
                if track_entry.would_be_dragged {
                    dragging_tracks.push_back(track_entry);
                } else {
                    not_dragging_tracks.push(track_entry);
                }
            }

            let mut current_y = ui.max_rect().top();
            for track_entry in not_dragging_tracks.into_iter().map(Some).chain([None]) {
                while dragging_tracks.front().is_some_and(|front| {
                    track_entry.as_ref().is_none_or(|track_entry| {
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

            track_list = TrackList::new(dragging_track_list, current_y);
        }

        Self {
            tab_has_focus: ctx.focused_tab() == Some(tab.id),
            dragging_would_succeed,
            track_list,
        }
    }

    fn ui_left_sidebar(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        ctx.ephemeral_state.track_drag.handle(
            |pos| pos,
            |prepared: &mut crate::util::Prepared<_, _>| {
                for track_entry in &mut self.track_list.list {
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
                        track_entry.track_ui.select,
                    );

                    if response.double_clicked() {
                        ctx.tabs
                            .get_or_create_tab::<super::pianoroll::PianoRollTab>(
                                ctx.state,
                                ctx.ui_state,
                            )
                            .select_track(Some(track_entry.track_id));

                        if let Some(patch_tab) = ctx.tabs.get_tab::<super::patch::PatchTab>() {
                            patch_tab.select_track(Some(track_entry.track_id));
                        }
                    }

                    // if not for the `!prepared.is_something_being_dragged()`, egui would think the rect at this position is hovered during dragging (which it's not)
                    track_entry.is_highlighted = !prepared.is_being_dragged()
                        && track_entry.track_ui.select.is()
                        || response.hovered();

                    ui.add_enabled_ui(
                        // if the track is being dragged, add disabled track headers to show where the tracks would be dropped
                        !(prepared.would_be_dragged(track_entry.track_ui.select)
                            && self.dragging_would_succeed),
                        |ui| {
                            track_entry.track_header(
                                &mut ctx.tracker,
                                ui,
                                rect,
                                track_entry.is_highlighted,
                                0,
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
                    for track_entry in &self.track_list.list {
                        if !track_entry.would_be_dragged {
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

                        ui.scope_builder(UiBuilder::new().layer_id(dragged_layer_id), |ui| {
                            ui.set_clip_rect(ui.ctx().screen_rect());
                            track_entry.track_header(
                                &mut ctx.tracker,
                                ui,
                                transformed_rect,
                                true,
                                1,
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
                ctx.tracker.add_weak(
                    move |ui_state: &mut crate::UiState,
                          _ephemeral_state: &mut crate::EphemeralState,
                          action: cubedaw_worker::command::ActionDirection| {
                        ui_state.show_root_track = match action {
                            ActionDirection::Forward => b,
                            ActionDirection::Reverse => !b,
                        };
                    },
                );
            }
        });
    }

    /// The non-track header area region thing. The place where you can see all clips.
    fn central_panel(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut crate::Context,
        view: &SongViewerPrepared,
    ) {
        let Self {
            ref mut track_list, ..
        } = *self;

        let mut track_entry_map: IdMap<Track, &TrackListEntry> = IdMap::new();
        for track_entry in &track_list.list {
            track_entry_map.insert(track_entry.track_id, track_entry);
        }

        let screen_rect = view.screen_rect;

        view.ui_background(ctx, ui, track_list.total_height());

        let bg_response = ui.response();

        let track_pos_to_screen_pos = |range: Range, entry: &TrackListEntry| -> Rect {
            Rect::from_x_y_ranges(
                view.song_range_to_screen_range(range),
                entry.actual_pos..=entry.actual_pos + entry.height,
            )
        };

        for (track_entry_index, track_entry) in track_list.list.iter().enumerate() {
            let highlighted = track_entry.is_highlighted;
            let visuals = if ctx
                .ephemeral_state
                .track_drag
                .would_be_dragged(track_entry.track_ui.select)
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

            // dbg!(
            //     ui.input(|i| i.pointer.hover_pos())
            //         .and_then(|p| track_list.entry_at_y(p.y))
            //         .map(|e| e.0)
            // );

            // clips
            ctx.ephemeral_state.clip_drag.handle(
                |Pos2 { x, y }| Track2DPos {
                    time: view.input_screen_x_to_song_x(x),
                    idx: {
                        if let Some((idx, _)) = track_list.entry_at_y(y) {
                            idx.try_into().unwrap()
                        } else {
                            // assume there are tracks of a certain height above and below the actual list
                            let track_height = DEFAULT_TRACK_HEIGHT;
                            if y < track_list.top {
                                ((y - track_list.top) / track_height).floor() as i32
                            } else {
                                ((y - track_list.bottom) / track_height) as i32
                                    + track_list.len() as i32
                            }
                        }
                    },
                },
                |drag| {
                    if bg_response.clicked() {
                        drag.deselect_all();
                    }

                    let track_id = track_entry.track_id;
                    let track = track_entry.track;

                    for (clip_range, clip_id, _clip) in track.clips() {
                        let clip_ui = track_entry.track_ui.clips.force_get(clip_id);

                        let mut clip_range = clip_range;
                        let mut track_entry = track_entry;

                        if drag.would_be_dragged(clip_ui.select)
                            && let Some(movement) = drag.movement()
                        {
                            clip_range += movement.time;
                            if movement.idx != 0 {
                                track_entry = &track_list.list[track_entry_index
                                    .saturating_add_signed(movement.idx as isize)
                                    .clamp(0, track_list.len() - 1)];
                            }
                        }

                        let clip_rect = track_pos_to_screen_pos(clip_range, track_entry);
                        let clip_response = ui
                            .allocate_rect(clip_rect, Sense::click_and_drag())
                            .on_hover_cursor(CursorIcon::Grab);

                        const SECTION_COLOR: Color32 = Color32::from_rgb(145, 0, 235);
                        ui.painter().rect(
                            clip_rect,
                            4.0,
                            match clip_ui.select {
                                Select::Select => SECTION_COLOR.gamma_multiply(0.7),
                                Select::Deselect => SECTION_COLOR.gamma_multiply(0.5),
                            },
                            Stroke::new(2.0, SECTION_COLOR),
                            StrokeKind::Inside,
                        );

                        drag.process_interaction(
                            clip_id.cast(),
                            &clip_response,
                            (track_id, clip_id),
                            clip_ui.select,
                        );
                    }
                },
            );
        }

        view.ui_top_bar(ctx, ui);

        view.ui_playhead(ctx, ui);
    }
}
