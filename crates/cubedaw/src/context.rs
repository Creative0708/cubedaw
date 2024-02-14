use std::any::Any;

use crate::screen::{viewer, PianoRollScreen, Screen, TrackScreen};

pub struct Context<'a> {
    pub paused: bool,

    pub state: &'a mut cubedaw_lib::State,
    pub tabs: Tabs<'a>,
}

pub struct Tabs<'a> {
    pub map: &'a mut cubedaw_lib::IdMap<(), viewer::Tab>,
}

impl<'a> Tabs<'a> {
    pub fn get_tabs<T: Screen>(&mut self) -> impl Iterator<Item = &mut T> {
        return self
            .map
            .iter_mut()
            .filter_map(|(_, tab)| (tab.as_mut() as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn get_tab<T: Screen>(&mut self) -> Option<&mut T> {
        return self
            .map
            .iter_mut()
            .find_map(|(_, tab)| (tab.as_mut() as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn has_tab<T: Screen>(&self) -> bool {
        return self
            .map
            .iter()
            .any(|(_, tab)| (tab.as_ref() as &dyn Any).is::<T>());
    }
}

impl<'a> Context<'a> {
    pub fn get_or_create_tab<T: Screen>(&mut self) -> &mut T {
        if self.tabs.has_tab::<T>() {
            return self.tabs.get_tab().unwrap();
        }

        let tab = T::create(self);

        let entry = self
            .tabs
            .map
            .entry(tab.id().transmute())
            .insert_entry(Box::new(tab));

        (entry.into_mut().as_mut() as &mut dyn Any)
            .downcast_mut()
            .unwrap()
    }
}
