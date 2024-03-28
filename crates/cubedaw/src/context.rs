use std::any::Any;

use cubedaw_lib::{Id, State};

use crate::{app::Tab, Screen, UiState};

pub struct Context<'a> {
    pub state: &'a mut State,
    pub ui_state: &'a mut UiState,
    pub tabs: Tabs<'a>,

    result: &'a mut ContextResult,
}

impl<'a> Context<'a> {
    pub fn new(
        state: &'a mut State,
        ui_state: &'a mut UiState,
        tabs: Tabs<'a>,
        result: &'a mut ContextResult,
    ) -> Self {
        Self {
            state,
            ui_state,
            tabs,

            result,
        }
    }

    pub fn get_or_create_tab<T: Screen>(&mut self) -> &mut T {
        if self.tabs.has_tab::<T>() {
            return self.tabs.get_tab().unwrap();
        }

        self.create_tab()
    }
    pub fn create_tab<T: Screen>(&mut self) -> &mut T {
        let tab = T::create(self);
        let id = tab.id();

        self.result.dock_queue.push(DockEvent::Create(id));

        self.tabs.map.insert(id, Box::new(tab));

        let tab = &mut **self.tabs.map.get_mut(&id).unwrap();

        (tab as &mut dyn Any).downcast_mut().unwrap()
    }
}

pub struct Tabs<'a> {
    pub map: &'a mut egui::ahash::HashMap<Id<Tab>, Tab>,
}

impl<'a> Tabs<'a> {
    pub fn get_tabs<T: Screen>(&mut self) -> impl Iterator<Item = &mut T> {
        return self
            .map
            .iter_mut()
            .filter_map(|(_, tab)| (&mut **tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn get_tab<T: Screen>(&mut self) -> Option<&mut T> {
        return self
            .map
            .iter_mut()
            .find_map(|(_, tab)| (&mut **tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn has_tab<T: Screen>(&self) -> bool {
        return self
            .map
            .iter()
            .any(|(_, tab)| (&**tab as &dyn Any).is::<T>());
    }
}

#[derive(Debug)]
pub struct ContextResult {
    dock_queue: Vec<DockEvent>,
}

impl ContextResult {
    pub fn new() -> Self {
        Self {
            dock_queue: Vec::new(),
        }
    }

    pub fn apply_dock_changes(&mut self, dock_state: &mut egui_dock::DockState<Id<Tab>>) {
        while let Some(event) = self.dock_queue.pop() {
            event.apply(dock_state);
        }
    }
}

#[derive(Debug)]
pub enum DockEvent {
    Create(Id<Tab>),
}

impl DockEvent {
    fn apply(self, dock_state: &mut egui_dock::DockState<Id<Tab>>) {
        match self {
            Self::Create(tab_id) => {
                let surface = dock_state.main_surface_mut();
                let root_node = surface.root_node_mut().expect("no root node found?");
                if root_node.is_leaf() && root_node.tabs_count() == 0 {
                    root_node.insert_tab(egui_dock::TabIndex(0), tab_id);
                } else {
                    surface.split_left(egui_dock::NodeIndex::root(), 0.4, vec![tab_id]);
                }
            }
        }
    }
}
