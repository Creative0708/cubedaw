
use egui::{Ui, Id, IdMap, ahash::HashMapExt, Layout, vec2, pos2, Rect, Sense, Rangef, Vec2};
use log::warn;

use crate::Context;

use super::Screen;


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
enum SectionType {
    SplitSection{
        first: Id,
        second: Id,
        direction: SplitDirection,

        split_pos: f32,
    },
    DisplaySection(Id),
}

#[derive(Debug)]
struct ScreenSection {
    pub rect: Rect,

    pub section_data: SectionType,

    pub id: Id,

    /// parent_id == id if this ScreenSection is the root section
    pub parent_id: Id,
}

const MIN_SECTION_SIZE: f32 = 50.0;

impl ScreenSection {
    pub fn new_display_section(rect: Rect, id: Id, parent_id: Id) -> Self {
        ScreenSection {
            rect,
            section_data: SectionType::DisplaySection(id),
            id,
            parent_id,
        }
    }
}

pub struct ScreenHandler {
    section_map: IdMap<ScreenSection>,
    display_map: IdMap<Box<dyn Screen>>,
    pub root_id: Id,

    last_used_id: u64,
    prev_slider_drag_coord: Option<f32>,

    screen_size: Vec2,
}

impl ScreenHandler {
    pub fn new(screen: Box<dyn Screen>) -> Self {
        let screen_id = screen.get_id();
        let root = ScreenSection::new_display_section(
            Rect { min: pos2(0.0, 0.0), max: pos2(1.0, 1.0) },
            screen_id,
            screen_id
        );

        let mut obj = Self {
            section_map: IdMap::new(),
            display_map: IdMap::new(),
            root_id: root.id,

            last_used_id: 256,
            prev_slider_drag_coord: None,

            screen_size: Vec2::ZERO,
        };

        obj.section_map.insert(root.id, root);
        obj.display_map.insert(screen.get_id(), screen);

        obj
    }

    pub fn split(&mut self, section_id: Id, direction: SplitDirection, invert: bool, new_screen: Box<dyn Screen>) {
        let split_section_id: Id = self.new_id();
        let new_id = new_screen.get_id();

        self.display_map.insert(new_id, new_screen);

        let section = self.section_map.remove(&section_id).unwrap();

        let rect = section.rect;

        let split_pos: f32 = 0.5;

        let rects = get_split_rects(direction, rect, split_pos);
        let rects = if invert { (rects.1, rects.0) } else { rects };

        let new_section = ScreenSection::new_display_section(rects.1, new_id, split_section_id);

        self.section_map.insert(split_section_id, ScreenSection {
            rect: section.rect,
            section_data: SectionType::SplitSection {
                first: section_id,
                second: new_id,
                direction: direction,
                split_pos: egui::emath::lerp(get_split_range(direction, rect), split_pos),
            },
            id: section_id,
            parent_id: section.parent_id,
        });
        self.section_map.insert(section_id, ScreenSection {
            rect: rects.0,
            section_data: section.section_data,
            id: section_id,
            parent_id: split_section_id,
        });
        self.section_map.insert(new_id, new_section);

        if section_id == self.root_id {
            self.root_id = split_section_id;
        }else {
            let parent_section = self.section_map.get_mut(&section.parent_id).unwrap();

            let SectionType::SplitSection { first, second, .. } = &mut parent_section.section_data else { panic!("Parent section is not a split section"); };

            if *first == section_id {
                *first = split_section_id;
            }else if *second == section_id {
                *second = split_section_id;
            }else {
                panic!("Parent of section does not have that section as a child");
            }
        }
    }

    pub fn update(&mut self, ctx: &Context, ui: &mut Ui) {
        self.screen_size = ui.max_rect().size();

        let mut stack = vec![self.root_id];

        while let Some(section_id) = stack.pop() {
            let section = self.section_map.get(&section_id).unwrap();

            let rect = section.rect;
            match &section.section_data {
                SectionType::SplitSection { first, second, .. } => {
                    stack.push(*first);
                    stack.push(*second);

                    self.handle_slider(section_id, ctx, ui);
                },
                SectionType::DisplaySection(id) => {
                    let screen = self.display_map.get_mut(id).unwrap();
                    
                    let child_rect = subrect(rect, ui.max_rect());

                    let mut child_ui = ui.child_ui_with_id_source(child_rect.shrink(4.0), Layout::default(), screen.get_id().with("child"));
                    child_ui.set_clip_rect(child_rect);

                    egui::CentralPanel::default().frame(egui::Frame::central_panel(child_ui.style()).inner_margin(10.0).rounding(10.0))
                        .show_inside(&mut child_ui, |ui| {
                            egui::ScrollArea::vertical().min_scrolled_height(0.0).auto_shrink([false, false]).show(ui, |ui| {
                                screen.update(ctx, ui);
                            });
                        });
                },
            }
        }
    }

    fn handle_slider(&mut self, section_id: Id, ctx: &Context, ui: &mut Ui){
        let section = self.section_map.get_mut(&section_id).unwrap();

        let SectionType::SplitSection { direction, split_pos, first, second } = section.section_data else { panic!("DisplaySection passed to handle_slider"); };
        let rect = section.rect;

        // Calculate the draggable area

        let split_rect = get_slider_rect(direction, rect, split_pos, ui.max_rect());

        // Handle interactions
        
        let slider_response = ui.allocate_rect(split_rect, Sense::click_and_drag());

        let mut set_cursor_icon = slider_response.hovered();
        if slider_response.drag_started() {
            if let Some(interact_pointer_pos) = slider_response.interact_pointer_pos() {
                self.prev_slider_drag_coord = Some(match direction {
                    SplitDirection::Horizontal => interact_pointer_pos.x / ui.max_rect().width(),
                    SplitDirection::Vertical => interact_pointer_pos.y / ui.max_rect().height(),
                });
            } else {
                warn!("No pointer pos on drag start");
            };
        }else if slider_response.dragged(){
            if let Some(pointer_pos) = slider_response.interact_pointer_pos() {
                if let Some(prev_pointer_coord) = self.prev_slider_drag_coord {
                    let diff = match direction {
                        SplitDirection::Horizontal => pointer_pos.x / ui.max_rect().width(),
                        SplitDirection::Vertical => pointer_pos.y / ui.max_rect().height(),
                    } - prev_pointer_coord;

                    
                    let final_offset = if diff < 0.0 {
                        self.get_maximum_offset(first, direction, true, diff)
                    }else {
                        -self.get_maximum_offset(second, direction, false, -diff)
                    };
                    self.update_dimensions(first, direction, true, final_offset);
                    self.update_dimensions(second, direction, false, -final_offset);

                    self.set_split_pos(section_id, split_pos + final_offset);

                    self.prev_slider_drag_coord = Some(prev_pointer_coord + final_offset);
                }
            }
            
            ctx.egui_ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            set_cursor_icon = false;
        }else if slider_response.drag_released() {
            self.prev_slider_drag_coord = None;
        }
        if set_cursor_icon {
            ctx.egui_ctx.set_cursor_icon(match direction {
                SplitDirection::Horizontal => egui::CursorIcon::ResizeHorizontal,
                SplitDirection::Vertical => egui::CursorIcon::ResizeVertical,
            });
        }

        {
            // Draw the little line segment thingies that indicate that there is a slider here

            // Refetch of the section is required otherwise the line segment thingies lag one frame behind
            let section = self.section_map.get_mut(&section_id).unwrap();
            let SectionType::SplitSection { direction, split_pos, .. } = section.section_data else { panic!("DisplaySection passed to handle_slider"); };
            
            let split_rect = get_slider_rect(direction, rect, split_pos, ui.max_rect());
            
            let painter = ui.painter();

            let split_center = split_rect.center().to_vec2();

            const SLIDER_SPACING: f32 = 1.5;
            const SLIDER_LENGTH: f32 = 4.0;

            let mut points = [
                vec2(-SLIDER_LENGTH, -SLIDER_SPACING),
                vec2( SLIDER_LENGTH, -SLIDER_SPACING),
                vec2(-SLIDER_LENGTH,  SLIDER_SPACING),
                vec2( SLIDER_LENGTH,  SLIDER_SPACING),
            ];

            if direction == SplitDirection::Horizontal {
                for val in points.iter_mut() {
                    *val = vec2(val.y, val.x);
                }
            }
            for val in points.iter_mut() {
                *val += split_center;
            }

            let stroke = egui::Stroke::new(2.0, ctx.egui_ctx.style().visuals.weak_text_color());
            painter.line_segment([points[0].to_pos2(), points[1].to_pos2()], stroke);
            painter.line_segment([points[2].to_pos2(), points[3].to_pos2()], stroke);
        }

    }

    /// A positive value for `target_offset` means expanding outwards, a negative value means shrinking inwards.
    fn get_maximum_offset(&mut self, section_id: Id, axis: SplitDirection, is_max_side: bool, target_offset: f32) -> f32{
        if target_offset == 0.0 {
            // We're not changing anything, so nothing needs to be done
            return 0.0;
        }

        let section = self.section_map.get(&section_id).unwrap();
        
        if let SectionType::SplitSection { first, second, direction, .. } = section.section_data {
            if axis == direction {
                let modified_section_id = if is_max_side { second } else { first };
                self.get_maximum_offset(modified_section_id, axis, is_max_side, target_offset)
            }else{
                f32::max(
                    self.get_maximum_offset(first, axis, is_max_side, target_offset),
                    self.get_maximum_offset(second, axis, is_max_side, target_offset)
                )
            }
        }else {
            if target_offset < 0.0 {
                let rect = section.rect;
                let (rect_dim, screen_dim) = match axis {
                    SplitDirection::Horizontal => (rect.width(), self.screen_size.x),
                    SplitDirection::Vertical => (rect.height(), self.screen_size.y),
                };
                f32::max(target_offset, MIN_SECTION_SIZE / screen_dim - rect_dim)
            }else {
                target_offset
            }
        }
    }

    /// A positive value for `target_offset` means expanding outwards, a negative value means shrinking inwards.
    fn update_dimensions(&mut self, section_id: Id, axis: SplitDirection, is_max_side: bool, target_offset: f32){
        if target_offset == 0.0 {
            // We're not changing anything, so nothing needs to be done
            return;
        }

        let section = self.section_map.get(&section_id).unwrap();
        
        if let SectionType::SplitSection { first, second, direction, .. } = section.section_data {
            if axis == direction {
                let modified_section_id = if is_max_side { second } else { first };
                self.update_dimensions(modified_section_id, axis, is_max_side, target_offset);
            }else{
                self.update_dimensions(first, axis, is_max_side, target_offset);
                self.update_dimensions(second, axis, is_max_side, target_offset);
            }
        }

        let section = self.section_map.get_mut(&section_id).unwrap();
        
        {
            let rect = &mut section.rect;

            match (axis, is_max_side) {
                (SplitDirection::Horizontal, false) => rect.set_left(rect.left() - target_offset),
                (SplitDirection::Horizontal, true) => rect.set_right(rect.right() + target_offset),
                (SplitDirection::Vertical, false) => rect.set_top(rect.top() - target_offset),
                (SplitDirection::Vertical, true) => rect.set_bottom(rect.bottom() + target_offset),
            }
        }
    }


    // Helper functions
    
    fn set_split_pos(&mut self, section_id: Id, new_split_pos: f32){
        let section = self.section_map.get_mut(&section_id).unwrap();
        let SectionType::SplitSection { split_pos, .. } = &mut section.section_data else { panic!("set_split_pos recieved display section id"); };
        *split_pos = new_split_pos;
    }

    fn new_id(&mut self) -> Id{
        let id = Id::new(self.last_used_id);
        self.last_used_id += 1;
        id
    }
}

// Helper functions

fn subrect(r1: Rect, r2: Rect) -> Rect{
    Rect {
        min: r2.lerp_inside(r1.min.to_vec2()),
        max: r2.lerp_inside(r1.max.to_vec2()),
    }
}

fn get_split_rects(direction: SplitDirection, rect: Rect, split_pos: f32) -> (Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => rect.split_left_right_at_fraction(split_pos),
        SplitDirection::Vertical => rect.split_top_bottom_at_fraction(split_pos),
    }
}

fn get_slider_rect(direction: SplitDirection, rect: Rect, split_pos: f32, max_rect: Rect) -> Rect{
    let unexpanded_slider_rect = match direction {
        SplitDirection::Horizontal => Rect::from_min_max(
            pos2(split_pos, rect.top()),
            pos2(split_pos, rect.bottom()),
        ),
        SplitDirection::Vertical => Rect::from_min_max(
            pos2(rect.left(), split_pos),
            pos2(rect.right(), split_pos),
        ),
    };
    subrect(unexpanded_slider_rect, max_rect).expand2(match direction {
        SplitDirection::Horizontal => vec2(7.5, -6.0),
        SplitDirection::Vertical => vec2(-6.0, 7.5),
    })
}

fn get_split_range(direction: SplitDirection, rect: Rect) -> Rangef {
    match direction {
        SplitDirection::Horizontal => rect.x_range(),
        SplitDirection::Vertical => rect.y_range(),
    }
}
