use cubedaw_lib::{Id, Track};

use crate::app::Tab;

pub struct TrackTab {
    id: Id<Tab>,
    track: Id<Track>,
}

impl crate::Screen for TrackTab {
    fn create(ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),
            track: ctx.state.tracks.create(Track::new()),
        }
    }

    fn id(&self) -> cubedaw_lib::Id<crate::app::Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Tracks".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        todo!()
    }
}

impl TrackTab {
    pub fn get_single_selected_track(&mut self) -> Id<Track> {
        self.track
    }
}
