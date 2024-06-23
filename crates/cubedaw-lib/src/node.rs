/// Miscellaneous node state used
pub trait NodeState: 'static + Sized + Send + Sync + Clone + PartialEq + Eq {
    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);
}

pub type DynNodeState = Box<dyn NodeStateWrapper>;

#[cfg(feature = "egui")]
mod ui {
    use std::any::TypeId;

    use egui::Rangef;

    use crate::NodeState;

    pub trait NodeUiContext {
        fn input_ui(&mut self, ui: &mut egui::Ui, name: &str, options: NodeInputUiOptions);
        fn output_ui(&mut self, ui: &mut egui::Ui, name: &str);
    }

    // TODO decide if this is necessary
    // #[derive(Clone, Debug)]
    // pub struct NodeVisuals {
    //     dragvalue_color
    // }
    // impl NodeVisuals {
    //     pub fn from_memory(ctx: &egui::Context) -> Self {
    //         ctx.data_mut(|d| d.get_persisted(id))
    //     }
    // }
    // impl Default for NodeVisuals {
    //     fn default() -> Self {
    //         Self {

    //         }
    //     }
    // }

    #[derive(Clone, Copy)]
    pub struct NodeInputUiOptions {
        pub display_fn: fn(f32) -> String,

        // The range values the dragvalue will show. if range.min == range.max, the dragvalue won't actually and the base value will be range.min.
        // TODO decide on whether a range where range.min > range.max is a violated invariant or just a logic error
        pub range: Rangef,

        // Self-explanatory.
        pub default_value: f32,

        // Whether the range is interactable. If false, the number won't render
        pub interactable: bool,
    }

    impl Default for NodeInputUiOptions {
        fn default() -> Self {
            Self {
                display_fn: |x| format!("{x:.2}"),
                range: Rangef::new(0.0, 1.0),
                default_value: 0.0,
                interactable: true,
            }
        }
    }

    impl NodeInputUiOptions {
        pub fn uninteractable() -> Self {
            Self {
                interactable: false,
                ..Default::default()
            }
        }
    }

    // to prevent soundness holes from manually implementing type_id on a struct
    mod sealed {
        pub trait Sealed {}
    }

    // TODO change this to unsafe possibly? actually just determine if the Any overhead is negligible
    pub trait NodeStateWrapper: 'static + sealed::Sealed + Send + Sync {
        fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);
        fn clone(&self) -> Box<dyn NodeStateWrapper>;
        fn eq(&self, rhs: &dyn NodeStateWrapper) -> bool;

        fn type_id(&self) -> TypeId;
    }

    impl<T: NodeState> sealed::Sealed for T {}
    impl<T: NodeState> NodeStateWrapper for T {
        fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext) {
            NodeState::ui(self, ui, ctx)
        }
        fn clone(&self) -> Box<dyn NodeStateWrapper> {
            Box::new(Clone::clone(self))
        }
        fn eq(&self, rhs: &dyn NodeStateWrapper) -> bool {
            let Some(rhs) = rhs.downcast_ref() else {
                panic!("eq called on rhs of different type");
            };
            PartialEq::eq(self, rhs)
        }

        fn type_id(&self) -> TypeId {
            TypeId::of::<T>()
        }
    }

    // Copy of `std::any::Any`. Replace this when trait upcasting is stabilized
    // https://doc.rust-lang.org/beta/unstable-book/language-features/trait-upcasting.html
    impl dyn NodeStateWrapper {
        pub fn downcast_ref<T: NodeStateWrapper + Sized>(&self) -> Option<&T> {
            if NodeStateWrapper::type_id(self) == TypeId::of::<T>() {
                Some(unsafe { &*(self as *const dyn NodeStateWrapper as *const T) })
            } else {
                None
            }
        }
        pub fn downcast_mut<T: NodeStateWrapper + Sized>(&mut self) -> Option<&T> {
            if NodeStateWrapper::type_id(self) == TypeId::of::<T>() {
                Some(unsafe { &*(self as *mut dyn NodeStateWrapper as *mut T) })
            } else {
                None
            }
        }
        pub fn downcast<T: NodeStateWrapper + Sized>(self: Box<Self>) -> Option<Box<T>> {
            if NodeStateWrapper::type_id(self.as_ref()) == TypeId::of::<T>() {
                Some(unsafe { Box::from_raw(Box::into_raw(self) as *mut T) })
            } else {
                None
            }
        }
    }

    impl std::fmt::Debug for dyn NodeStateWrapper {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("<dyn NodeUiWrapper>")
        }
    }

    impl Clone for Box<dyn NodeStateWrapper> {
        fn clone(&self) -> Self {
            NodeStateWrapper::clone(self.as_ref())
        }
    }
    impl PartialEq for dyn NodeStateWrapper {
        fn eq(&self, other: &Self) -> bool {
            NodeStateWrapper::eq(self, other)
        }
    }
    impl Eq for dyn NodeStateWrapper {}
}

#[cfg(feature = "egui")]
pub use ui::{NodeInputUiOptions, NodeStateWrapper, NodeUiContext};
