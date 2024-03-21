use std::any::Any;

use cubedaw_lib::{Id, State};

use crate::{app, Screen};

pub struct Context<'a> {
    pub state: &'a mut State,
    pub tabs: Tabs<'a>,
}

impl<'a> Context<'a> {
    pub fn new(state: &'a mut State, tabs: Tabs<'a>) -> Self {
        Self { state, tabs }
    }

    pub fn get_or_create_tab<T: Screen>(&mut self) -> &mut T {
        if self.tabs.has_tab::<T>() {
            return self.tabs.get_tab().unwrap();
        }

        let tab = T::create(self);

        let tab = self.tabs.map.entry(tab.id()).or_insert(Box::new(tab));

        (&mut *tab as &mut dyn Any).downcast_mut().unwrap()
    }
}

pub struct Tabs<'a> {
    pub map: &'a mut egui::ahash::HashMap<Id<app::Tab>, app::Tab>,
}

impl<'a> Tabs<'a> {
    pub fn get_tabs<T: Screen>(&mut self) -> impl Iterator<Item = &mut T> {
        return self
            .map
            .iter_mut()
            .filter_map(|(_, tab)| (&mut *tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn get_tab<T: Screen>(&mut self) -> Option<&mut T> {
        return self
            .map
            .iter_mut()
            .find_map(|(_, tab)| (&mut *tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn has_tab<T: Screen>(&self) -> bool {
        return self
            .map
            .iter()
            .any(|(_, tab)| (&*tab as &dyn Any).is::<T>());
    }
}
