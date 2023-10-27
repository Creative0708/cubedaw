use egui::{Vec2, LayerId, Ui, Id, IdMap, ahash::HashMapExt, Area, SidePanel, Layout, vec2, pos2, Rect, Pos2};

use super::Screen;


pub enum SplitDirection {
    Horizontal,
    Vertical,
}

enum SectionType {
    SplitSection{
        first: Box<ScreenSection>,
        second: Box<ScreenSection>,
        direction: SplitDirection,

        split_pos: f32,
    },
    DisplaySection(egui::Id),
}

struct ScreenSection {
    pub rect: egui::Rect,

    pub section_data: SectionType,
}

pub struct ScreenHandler {
    root: ScreenSection,
    section_map: IdMap<Box<dyn Screen>>,
}

impl ScreenHandler {
    pub fn new(screen: Box<dyn Screen>) -> Self {
        let mut obj = Self {
            root: ScreenSection {
                rect: Rect { min: pos2(0.0, 0.0), max: pos2(1.0, 1.0) },
                section_data: SectionType::DisplaySection(screen.get_id()),
            },
            section_map: IdMap::new(),
        };

        obj.section_map.insert(screen.get_id(), screen);

        obj
    }

    fn find(&self, screen_id: Id) -> Option<&ScreenSection> {
        let mut stack = vec![&self.root];

        while let Some(section) = stack.pop() {
            match &section.section_data {
                SectionType::SplitSection { first, second, .. } => {
                    stack.push(&*first);
                    stack.push(&*second);
                },
                SectionType::DisplaySection(id) => {
                    if *id == screen_id {
                        return Some(section);
                    }
                },
            }
        }

        None
    }

    fn find_mut(&mut self, screen_id: Id) -> Option<&mut ScreenSection> {
        let mut stack = vec![&mut self.root];

        while let Some(section) = stack.pop() {
            match &mut section.section_data {
                SectionType::SplitSection { first, second, .. } => {
                    stack.push(&mut *first);
                    stack.push(&mut *second);
                },
                SectionType::DisplaySection(id) => {
                    if *id == screen_id {
                        return Some(section);
                    }
                },
            }
        }

        None
    }

    pub fn split(&mut self, screen_id: Id, direction: SplitDirection, invert: bool, new_screen: Box<dyn Screen>){
        self.section_map.insert(new_screen.get_id(), new_screen);

        let old_section = self.find(screen_id).unwrap();


    }

    pub fn update(&mut self, ctx: &crate::Context, ui: &mut egui::Ui) {

        let mut stack = vec![&mut self.root];

        while let Some(section) = stack.pop() {
            let rect = section.rect;
            match &mut section.section_data {
                SectionType::SplitSection { first, second, direction, split_pos } => {
                    stack.push(&mut *first);
                    stack.push(&mut *second);

                    let split_rect = match direction {
                        SplitDirection::Horizontal => Rect::from_min_max(
                            pos2(*split_pos, rect.top()),
                            pos2(*split_pos, rect.bottom()),
                        ),
                        SplitDirection::Vertical => Rect::from_min_max(
                            pos2(rect.left(), *split_pos),
                            pos2(rect.right(), *split_pos),
                        ),
                    };
                    
                    if ui.rect_contains_pointer(split_rect) {
                        ctx.egui_ctx.set_cursor_icon(match direction {
                            SplitDirection::Horizontal => egui::CursorIcon::ResizeHorizontal,
                            SplitDirection::Vertical => egui::CursorIcon::ResizeVertical,
                        });
                    }
                },
                SectionType::DisplaySection(id) => {
                    let screen = self.section_map.get_mut(id).unwrap();
                    
                    let child_rect = subrect(rect, ui.max_rect()).shrink(5.0);

                    let mut child_ui = ui.child_ui(child_rect.shrink(5.0), Layout::default());
                    child_ui.set_clip_rect(child_rect);
                    
                    egui::CentralPanel::default().frame(egui::Frame::central_panel(ui.style()).inner_margin(5.0))
                        .show_inside(ui, |ui| { screen.update(ctx, &mut child_ui); });
                },
            }
        }
    }

}

fn subrect(r1: Rect, r2: Rect) -> Rect{
    let size = r2.size();
    Rect {
        min: r2.min + r1.min.to_vec2() * size,
        max: r2.min + r1.max.to_vec2() * size,
    }
}