mod drag_value;
pub use drag_value::{DefaultValueDisplay, DragValue, ValueHandler, ValueHandlerContext};
mod editable_label;
pub use editable_label::{EditableLabel, EditableLabelState};

bitflags::bitflags! {
    /// Generic input modifiers that can be rebound (in the future, that is. key remapping isn't available right now)
    ///
    /// The modifiers should have semantic meanings. Why? Because I said so and this is my project.
    ///
    /// This text is to make Zed not present the above text as a header. I don't know why the last piece of text in a bitflags triggers the header formatting.
    pub struct InputModifiers: u8 {
        /// "Alternate" modifier. By default, bound to the shift keys.
        ///
        /// This means that the action performed accomplishes the same thing, but is done in a slightly different way. For example, in a snapped drag value, the alternate behavior would be to not snap.
        ///
        /// This is also applicable for noninteractive stuff. For example, a `DragValue` could should one representation of a value at rest and another representation when the alternate key is pressed. The sky's the limit!
        const ALTERNATE = 1 << 0;
    }
}
impl InputModifiers {
    pub fn read_from_egui_input(input: &egui::InputState) -> Self {
        let mut this = Self::empty();
        // TODO: these are currently hardcoded. these should be remappable in the future
        if input.modifiers.shift {
            this |= Self::ALTERNATE
        }
        this
    }
}
