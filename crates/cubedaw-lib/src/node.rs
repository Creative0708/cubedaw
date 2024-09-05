/// Miscellaneous node state used for node configuration.
/// Should be relatively cheap to clone as it will be cloned every ui frame.
pub trait NodeState: 'static + Sized + Send + Sync + Clone + PartialEq + Eq {
    fn title(&self) -> Cow<'_, str>;
    #[cfg(feature = "egui")]
    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);
}

pub type DynNodeState = Box<dyn NodeStateWrapper>;

pub type DynNode = Box<dyn NodeWrapper>;

#[derive(Clone, Copy)]
pub enum DataSource<'a> {
    Const(BufferType),
    Buffer(&'a [BufferType]),
}
impl std::ops::Index<u32> for DataSource<'_> {
    type Output = BufferType;
    fn index(&self, index: u32) -> &Self::Output {
        match self {
            Self::Const(val) => val,
            Self::Buffer(buf) => &buf[index as usize],
        }
    }
}

pub enum DataDrain<'a> {
    Disconnected,
    NodeInput(&'a [std::cell::Cell<BufferType>]),
}
impl DataDrain<'_> {
    pub fn set(&self, i: u32, val: BufferType) {
        match self {
            Self::Disconnected => (),
            Self::NodeInput(buf) => {
                buf[i as usize].set(val);
            }
        }
    }
}

pub trait NodeContext<'a> {
    fn sample_rate(&self) -> u32;
    fn buffer_size(&self) -> u32;
    fn input(&self, index: u32) -> DataSource<'_>;
    fn output(&self, index: u32) -> DataDrain<'_>;

    fn property(&self, property: NoteProperty) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NoteProperty(pub NonZeroU32);
impl NoteProperty {
    pub const fn new(val: u32) -> Option<Self> {
        match NonZeroU32::new(val) {
            None => None,
            Some(val) => Some(Self(val)),
        }
    }
    /// # Safety
    /// `val != 0` otherwise it's UB. You know the drill.
    pub const unsafe fn new_unchecked(val: u32) -> Self {
        Self(unsafe { NonZeroU32::new_unchecked(val) })
    }

    const fn new_or_panic(val: u32) -> Self {
        match Self::new(val) {
            Some(p) => p,
            None => panic!("0 passed to NoteProperty::new_or_panic"),
        }
    }
    pub const PITCH: Self = Self::new_or_panic(1);
    pub const TIME_SINCE_START: Self = Self::new_or_panic(2);
    pub const BEATS_SINCE_START: Self = Self::new_or_panic(3);
}

pub trait Node: 'static + Sized + Send + Clone {
    // the reason for the whole Self::State thing is to have a way to have the ui thread render without waiting for thread synchronization
    // (which could cause very bad ui delays.)
    // also we need to be able to serialize the state to disk and this provides a convenient struct to do so
    type State: NodeState;

    fn new() -> Self;
    fn new_state(creation_ctx: NodeCreationContext<'_>) -> Self::State;

    fn process(&mut self, state: &Self::State, ctx: &mut dyn NodeContext<'_>);
}

mod sealed {
    pub trait Sealed {}
}
/// Object-safe wrapper for `Node`. See [`Node`] for the actual functionality.
pub trait NodeWrapper: 'static + Send + sealed::Sealed + std::any::Any {
    fn process(&mut self, state: &dyn NodeStateWrapper, ctx: &mut dyn NodeContext<'_>);

    fn clone(&self) -> Box<dyn NodeWrapper>;
}

impl<T: Node> sealed::Sealed for T {}
impl<T: Node> NodeWrapper for T {
    fn process(&mut self, state: &dyn NodeStateWrapper, ctx: &mut dyn NodeContext<'_>) {
        self.process(state.downcast_ref().expect("mismatched state type"), ctx)
    }

    fn clone(&self) -> Box<dyn NodeWrapper> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeWrapper> {
    fn clone(&self) -> Self {
        NodeWrapper::clone(self.as_ref())
    }
}
impl std::fmt::Debug for dyn NodeWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("dyn NodeWrapper { .. }")
    }
}

impl dyn NodeWrapper {
    pub fn downcast_ref<T: NodeWrapper + Sized>(&self) -> Option<&T> {
        if std::any::Any::type_id(self) == std::any::TypeId::of::<T>() {
            Some(unsafe { &*(self as *const dyn NodeWrapper as *const T) })
        } else {
            None
        }
    }
    pub fn downcast_mut<T: NodeWrapper + Sized>(&mut self) -> Option<&mut T> {
        if std::any::Any::type_id(self) == std::any::TypeId::of::<T>() {
            Some(unsafe { &mut *(self as *mut dyn NodeWrapper as *mut T) })
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct NodeCreationContext<'a> {
    pub alias: Option<std::borrow::Cow<'a, str>>,
}

#[cfg(feature = "egui")]
mod ui {
    use std::{any::TypeId, borrow::Cow};

    use egui::{Rangef, WidgetText};

    use crate::NodeState;

    pub trait NodeUiContext {
        fn input_ui(&mut self, ui: &mut egui::Ui, name: &str, options: NodeInputUiOptions);
        fn output_ui(&mut self, ui: &mut egui::Ui, name: &str);
    }

    pub struct NodeInputUiOptions<'a> {
        pub display: &'a dyn ValueHandler,

        /// The range of values the dragvalue will show. if range.min == range.max, the dragvalue won't actually display a filled percentage.
        /// A range where `range.min > range.max` is a logic error. Something something panics aborts but not UB blah blah
        pub display_range: Rangef,

        /// The range of draggable values. If this is `Rangef::EVERYTHING`, the range is unbounded.
        pub range: Rangef,

        /// Drag speed multiplier. If `None`, the pos will lock to the cursor when starting a drag.
        pub base_drag_speed: Option<f32>,

        /// Self-explanatory.
        pub default_value: f32,

        /// Whether the range is interactable. If false, the number won't render.
        pub interactable: bool,
    }

    impl Default for NodeInputUiOptions<'_> {
        fn default() -> Self {
            struct DefaultValueDisplay;
            impl ValueHandler for DefaultValueDisplay {
                fn to_input(&self, val: f32) -> String {
                    format!("{val:.2}")
                }
                fn parse_input(&self, str: &str) -> Option<f32> {
                    str.parse().ok()
                }
                fn snap(&self, val: f32) -> f32 {
                    (val * 100.0).round() * 0.01
                }
            }
            Self {
                display: &DefaultValueDisplay,
                display_range: Rangef::new(0.0, 1.0),
                range: Rangef::new(0.0, 1.0),
                base_drag_speed: None,
                default_value: 0.0,
                interactable: true,
            }
        }
    }

    impl NodeInputUiOptions<'_> {
        pub fn uninteractable() -> Self {
            Self {
                interactable: false,
                ..Default::default()
            }
        }
        pub fn pitch() -> Self {
            struct PitchDisplay;
            impl PitchDisplay {
                fn get_parts(&self, val: f32) -> (String, String) {
                    let pitch = val * 12.0;
                    let rounded_pitch = pitch.round();
                    let integer_pitch = rounded_pitch as i32;
                    let difference_cents = ((pitch - rounded_pitch) * 100.0).round() as i32;

                    let note_str = [
                        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                    ][integer_pitch.rem_euclid(12) as usize];
                    let octave = integer_pitch.div_euclid(12) + 4;

                    let note_name = format!("{note_str}{octave}");

                    (
                        note_name,
                        if difference_cents != 0 {
                            format!("{difference_cents:+03}")
                        } else {
                            "".into()
                        },
                    )
                }
            }
            impl ValueHandler for PitchDisplay {
                fn to_display(&self, val: f32) -> WidgetText {
                    let (note_name, difference) = self.get_parts(val);

                    (note_name + &difference).into()
                }
                fn to_input(&self, val: f32) -> String {
                    let (note_name, difference) = self.get_parts(val);
                    note_name + &difference
                }
                fn parse_input(&self, str: &str) -> Option<f32> {
                    if let Ok(val) = str.parse::<f32>() {
                        Some(val / 12.0)
                    } else {
                        let (note_name, difference) = match str.find(['+', '-']) {
                            Some(index) => str.split_at(index),
                            None => (str, ""),
                        };
                        let (note_name, difference) = (note_name.trim(), difference.trim());
                        let (note_offset, rest) = match note_name.as_bytes() {
                            [b'B', rest @ ..] => (11, rest),
                            [b'A', b'#', rest @ ..] => (10, rest),
                            [b'A', rest @ ..] => (9, rest),
                            [b'G', b'#', rest @ ..] => (8, rest),
                            [b'G', rest @ ..] => (7, rest),
                            [b'F', b'#', rest @ ..] => (6, rest),
                            [b'F', rest @ ..] => (5, rest),
                            [b'E', rest @ ..] => (4, rest),
                            [b'D', b'#', rest @ ..] => (3, rest),
                            [b'D', rest @ ..] => (2, rest),
                            [b'C', b'#', rest @ ..] => (1, rest),
                            [b'C', rest @ ..] => (0, rest),
                            _ => return None,
                        };

                        let octave = match std::str::from_utf8(rest)
                            .expect("unreachable, we only removed ascii characters from the front")
                            .parse::<i32>()
                        {
                            Ok(x) => x,
                            Err(err) if *err.kind() == std::num::IntErrorKind::Empty => 3,
                            _ => return None,
                        };

                        let pitch = (octave - 4) * 12 + note_offset;

                        let difference = match difference.parse::<f32>() {
                            Ok(val) => val,
                            Err(_) if difference.is_empty() => 0.0,
                            Err(_) => return None,
                        };

                        let pitch_with_difference = pitch as f32 + difference * 0.01;

                        Some(pitch_with_difference / 12.0)
                    }
                }
                fn snap(&self, val: f32) -> f32 {
                    (val * 12.0).round() / 12.0
                }
            }
            Self {
                display: &PitchDisplay,
                display_range: Rangef::new(-2.0, 4.0),
                range: Rangef::EVERYTHING,

                ..Default::default()
            }
        }
    }

    pub trait ValueHandler {
        fn to_display(&self, val: f32) -> WidgetText {
            self.to_input(val).into()
        }
        fn to_input(&self, val: f32) -> String;
        // TODO implement expression evaluator based off of https://crates.io/crates/meval or the like
        fn parse_input(&self, str: &str) -> Option<f32>;

        fn snap(&self, val: f32) -> f32;

        fn default_value(&self) -> f32 {
            0.0
        }
    }

    // to prevent soundness holes from manually implementing type_id on a struct
    mod sealed {
        pub trait Sealed {}
    }

    // TODO change this to unsafe possibly? actually just determine if the Any overhead is negligible
    pub trait NodeStateWrapper: 'static + sealed::Sealed + Send + Sync {
        fn title(&self) -> Cow<'_, str>;
        fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn NodeUiContext);

        fn clone(&self) -> Box<dyn NodeStateWrapper>;
        fn eq(&self, rhs: &dyn NodeStateWrapper) -> bool;

        fn type_id(&self) -> TypeId;
    }

    impl<T: NodeState> sealed::Sealed for T {}
    impl<T: NodeState> NodeStateWrapper for T {
        fn title(&self) -> Cow<'_, str> {
            NodeState::title(self)
        }
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

use std::{borrow::Cow, num::NonZeroU32};

#[cfg(feature = "egui")]
pub use ui::{NodeInputUiOptions, NodeStateWrapper, NodeUiContext, ValueHandler};

use crate::BufferType;
