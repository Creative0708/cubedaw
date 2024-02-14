use cubedaw_lib::{
    math::subrect,
    track::{Section, Track, TrackData},
    Id, IdCorrespondenceMap as _, IdMap, IdSet,
};
use egui::{
    ahash::{HashMapExt, HashSetExt},
    epaint::PathShape,
    pos2, vec2, Align, CentralPanel, Color32, CursorIcon, Frame, Layout, Margin, Rect, Rounding,
    ScrollArea, Sense, Shape, SidePanel, Stroke, TopBottomPanel, Ui, Vec2, WidgetText,
};

use crate::Context;

use super::{PianoRollScreen, Screen};

#[derive(Debug)]
pub struct TrackScreen {
    id: Id<TrackScreen>,

    ui_data: IdMap<Track, TrackUIData>,
}

impl TrackScreen {
    pub fn new(id: Id<TrackScreen>) -> Self {
        Self {
            id,
            ui_data: IdMap::new(),
        }
    }

    fn top_bar(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let play_button = crate::widget::PainterButton::new(|painter, _selected, visuals| {
                let clip_rect = painter.clip_rect();
                let icon_color = visuals.text_color();

                if ctx.paused {
                    // Play button
                    painter.add(Shape::Path(PathShape::convex_polygon(
                        vec![
                            clip_rect.lerp_inside(vec2(0.28, 0.21)),
                            clip_rect.lerp_inside(vec2(0.8, 0.5)),
                            clip_rect.lerp_inside(vec2(0.28, 0.79)),
                        ],
                        icon_color,
                        Stroke::NONE,
                    )));
                } else {
                    // Pause button
                    painter.rect_filled(
                        subrect(
                            Rect {
                                min: pos2(0.24, 0.2),
                                max: pos2(0.4, 0.8),
                            },
                            clip_rect,
                        ),
                        Rounding::ZERO,
                        icon_color,
                    );
                    painter.rect_filled(
                        subrect(
                            Rect {
                                min: pos2(0.6, 0.2),
                                max: pos2(0.76, 0.8),
                            },
                            clip_rect,
                        ),
                        Rounding::ZERO,
                        icon_color,
                    );
                    PathShape::convex_polygon(
                        vec![
                            clip_rect.lerp_inside(vec2(0.28, 0.21)),
                            clip_rect.lerp_inside(vec2(0.8, 0.5)),
                            clip_rect.lerp_inside(vec2(0.28, 0.79)),
                        ],
                        icon_color,
                        Stroke::NONE,
                    );
                };
            });
            if ui.add(play_button).clicked() {
                ctx.paused = !ctx.paused;
            }
        });
    }

    fn tracks(&mut self, ui: &mut Ui, ctx: &mut Context, scroll_max_rect: Rect) {
        SidePanel::left(self.id.with("left_track_panel"))
            .frame(Frame::side_top_panel(ui.style()).inner_margin(0.0))
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let ui_xrange = ui.max_rect().x_range();

                let mut opened_track_id = None;

                for (row, track_id) in ctx.state.tracks.iter().copied().enumerate() {
                    let track_ui_data = self
                        .ui_data
                        .entry(track_id)
                        .or_insert_with(|| TrackUIData::new(track_id));

                    let track = ctx.state.track_map.id_get_mut(track_id);

                    // TODO optimize rendering so that non-visible tracks are not rendered

                    let mut is_hovering_over_resize_bar = false;
                    let mut is_resizing = false;

                    let egui_track_id: egui::Id = track_id.into();
                    let resize_id = egui_track_id.with("resize");

                    if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                        if ui.memory(|mem| {
                            !mem.is_anything_being_dragged() || mem.is_being_dragged(resize_id)
                        }) {
                            let is_on_top = ui
                                .ctx()
                                .layer_id_at(pointer)
                                .map_or(true, |top_layer_id| top_layer_id == ui.layer_id());

                            is_hovering_over_resize_bar = ui.clip_rect().contains(pointer)
                                && is_on_top
                                && ui_xrange.contains(pointer.x)
                                && (ui.cursor().top() + track_ui_data.height - pointer.y).abs()
                                    <= ui.style().interaction.resize_grab_radius_side;

                            if ui.input(|i| i.pointer.any_pressed() && i.pointer.any_down())
                                && is_hovering_over_resize_bar
                            {
                                ui.memory_mut(|mem| mem.set_dragged_id(resize_id));
                            }
                            is_resizing = ui.memory(|mem| mem.is_being_dragged(resize_id));

                            if is_resizing {
                                let target_height = (pointer.y - ui.cursor().top()).max(16.0);
                                track_ui_data.height = target_height;
                            }
                        }
                    }

                    let response =
                        ui.allocate_ui(vec2(ui_xrange.span(), track_ui_data.height), |ui| {
                            let max_rect = ui.max_rect();

                            if !scroll_max_rect.intersects(max_rect) {
                                ui.advance_cursor_after_rect(max_rect);
                                return (false, false);
                            }

                            ui.set_clip_rect(scroll_max_rect.intersect(max_rect));

                            let mut child_ui = ui
                                .child_ui(max_rect.shrink(8.0), Layout::left_to_right(Align::Min));

                            let header_res =
                                ui.interact(max_rect, egui_track_id.with("header"), Sense::click());

                            if row % 2 == 0 {
                                ui.painter().rect_filled(
                                    Rect::from_x_y_ranges(ui_xrange, max_rect.y_range()),
                                    Rounding::ZERO,
                                    ui.style().visuals.faint_bg_color,
                                );
                            }

                            if header_res.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                ui.painter().rect_filled(
                                    max_rect,
                                    Rounding::ZERO,
                                    ui.style().visuals.widgets.hovered.bg_fill,
                                );
                            } else if track_ui_data.selected {
                                child_ui.painter().rect_filled(
                                    max_rect,
                                    Rounding::ZERO,
                                    ui.style().visuals.widgets.active.bg_fill,
                                );
                            }

                            child_ui.heading(track.name.as_str());

                            if is_hovering_over_resize_bar {
                                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                            }
                            let stroke = if is_resizing {
                                ui.style().visuals.widgets.active.fg_stroke
                            } else if is_hovering_over_resize_bar {
                                ui.style().visuals.widgets.hovered.fg_stroke
                            } else {
                                ui.style().visuals.widgets.noninteractive.bg_stroke
                            };
                            ui.painter().line_segment(
                                [max_rect.left_bottom(), max_rect.right_bottom()],
                                stroke,
                            );

                            ui.advance_cursor_after_rect(max_rect);

                            (header_res.double_clicked(), header_res.clicked())
                        });

                    let (should_open_track, track_selected) = response.inner;

                    if should_open_track {
                        opened_track_id = Some(track_id);
                    }
                    if track_selected {
                        if !ui.input(|i| i.modifiers.shift) {
                            for (&id, ui_data) in &mut self.ui_data {
                                ui_data.selected = id == track_id;
                            }
                        } else {
                            track_ui_data.selected = true;
                        }
                    }
                }
                if let Some(track_id) = opened_track_id {
                    ctx.get_or_create_tab::<PianoRollScreen>()
                        .select(Some((track_id, None)));
                }
            });

        CentralPanel::default()
            .frame(
                Frame::central_panel(ui.style())
                    .inner_margin(0.0)
                    .fill(ui.visuals().window_fill),
            )
            .show_inside(ui, |ui| {
                ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .drag_to_scroll(false)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;

                        let ui_xrange = ui.max_rect().x_range();
                        for (row, track_id) in ctx.state.tracks.iter().cloned().enumerate() {
                            let &TrackUIData {
                                selected, height, ..
                            } = self.ui_data.get(&track_id).unwrap();

                            ui.allocate_ui(vec2(ui.max_rect().width(), height), |ui| {
                                let max_rect = ui.max_rect();

                                if !scroll_max_rect.intersects(max_rect) {
                                    ui.advance_cursor_after_rect(max_rect);
                                    return;
                                }

                                ui.set_clip_rect(scroll_max_rect.intersect(max_rect));

                                if selected {
                                    ui.painter().rect_filled(
                                        max_rect,
                                        Rounding::ZERO,
                                        ui.style().visuals.widgets.active.bg_fill,
                                    );
                                }
                                if row % 2 == 0 {
                                    ui.painter().rect_filled(
                                        Rect::from_x_y_ranges(ui_xrange, ui.max_rect().y_range()),
                                        Rounding::ZERO,
                                        ui.style().visuals.faint_bg_color,
                                    );
                                }
                                self.track_contents(ctx.state.track_map.id_get_mut(track_id), ui);
                                ui.advance_cursor_after_rect(ui.max_rect());
                            });
                        }
                    });
            });

        self.cleanup(ctx);
    }

    fn track_contents(&mut self, track: &mut Track, ui: &mut egui::Ui) {
        ui.label(format!(
            "Contents of {}. Nothing here yet",
            track.name.as_str()
        ));
    }

    fn cleanup(&mut self, ctx: &mut Context) {
        if self.ui_data.len() > ctx.state.tracks.len() {
            let deleted_tracks: Vec<Id<Track>> = self
                .ui_data
                .iter()
                .filter_map(|(&id, data)| {
                    if ctx.state.track_map.id_has(data.track) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();
            for id in deleted_tracks {
                self.ui_data.remove(&id);
            }
        }
        for (id, ui_data) in &mut self.ui_data {}
    }

    pub fn get_single_selected_track_and_section(
        &mut self,
        track_map: &mut IdMap<Track, Track>,
    ) -> Option<(Id<Track>, Option<Id<Section>>)> {
        let mut found: Option<(Id<Track>, Option<Id<Section>>)> = None;

        #[allow(overlapping_range_endpoints)]
        for (&track_id, ui_data) in &self.ui_data {
            if ui_data.selected {
                match (found, ui_data.selected_sections.len()) {
                    (Some((_, Some(_))), 1..) => return None,
                    (None, 2..) => found = Some((ui_data.track, None)),
                    (Some((_, None)) | None, ..=1) => {
                        let track = track_map.id_get(track_id);
                        let TrackData::SynthesizerTrack(track_data) = &track.track_data else {
                            todo!()
                        };
                        found = Some((
                            track_id,
                            ui_data.selected_sections.iter().next().map(|id| *id),
                        ));
                    }
                    _ => (),
                }
            }
        }
        found
    }

    // /// Gets the one selected section of the track screen.
    // /// If there are no tracks selected or multiple tracks selected, returns `None`.
    // pub fn get_single_selected_section(&self) -> Option<Id> {

    // }
}

impl Screen for TrackScreen {
    fn id(&self) -> Id<()> {
        self.id.transmute()
    }

    fn title(&self) -> WidgetText {
        "Track View".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        TopBottomPanel::top(self.id.with("top_menu"))
            .frame(
                egui::Frame::side_top_panel(ui.style()).inner_margin(Margin {
                    left: 0.0,
                    right: 0.0,
                    top: 0.0,
                    bottom: 8.0,
                }),
            )
            .show_inside(ui, |ui| self.top_bar(ctx, ui));
        CentralPanel::default()
            .frame(Frame::central_panel(ui.style()).fill(Color32::TRANSPARENT))
            .show_inside(ui, |ui| {
                let scroll_max_rect = ui.max_rect();
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .drag_to_scroll(false)
                    .show(ui, |ui| {
                        self.tracks(ui, ctx, scroll_max_rect);
                    })
            });
    }

    fn create(_ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self::new(Id::arbitrary())
    }
}

#[derive(Debug)]
struct TrackUIData {
    track: Id<Track>,
    height: f32,
    selected: bool,
    selected_sections: IdSet<Section>,
}

impl TrackUIData {
    fn new(track: Id<Track>) -> Self {
        Self {
            track,
            height: 64.0,
            selected: false,
            selected_sections: IdSet::new(),
        }
    }
}
