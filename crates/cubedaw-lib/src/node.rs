pub trait NodeState: 'static + Sized + Send + Clone + PartialEq + Eq {
    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);
}

#[cfg(feature = "egui")]
mod ui {
    use std::any::Any;

    use crate::NodeState;

    pub trait NodeUiContext {
        fn input_ui(&mut self, ui: &mut egui::Ui, name: &str);
        fn output_ui(&mut self, ui: &mut egui::Ui, name: &str);
    }

    // TODO change this to unsafe possibly? actually just determine if the Any overhead is negligible
    pub trait NodeStateWrapper: Send + Any {
        fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);
        fn clone(&self) -> Box<dyn NodeStateWrapper>;
        fn eq(&self, rhs: &dyn NodeStateWrapper) -> bool;

        // TODO change this into a single vtable entry for type_id() and implement like trait AnyExt {} for that
        fn as_any(&self) -> &dyn Any;
        fn into_any(self: Box<Self>) -> Box<dyn Any>;
    }

    impl<T: NodeState> NodeStateWrapper for T {
        fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
            NodeState::ui(self, ui, ctx)
        }
        fn clone(&self) -> Box<dyn NodeStateWrapper> {
            Box::new(Clone::clone(self))
        }
        fn eq(&self, rhs: &dyn NodeStateWrapper) -> bool {
            let rhs = rhs.as_any();
            let Some(rhs) = rhs.downcast_ref() else {
                panic!("eq called on rhs of different type");
            };
            PartialEq::eq(self, rhs)
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn into_any(self: Box<Self>) -> Box<dyn Any> {
            self
        }
    }

    impl std::fmt::Debug for dyn NodeStateWrapper {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("<dyn NodeUiWrapper>")
        }
    }
}

use std::any::Any;

#[cfg(feature = "egui")]
pub use ui::{NodeStateWrapper, NodeUiContext};
