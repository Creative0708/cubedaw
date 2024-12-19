use std::{any::TypeId, borrow::Cow};

use egui::{Rangef, WidgetText};

use crate::widget::{ValueHandler, ValueHandlerContext};

pub trait NodeUiContext {
    // TODO: currently this only takes into account the position of the inputs to determine what inputs are the same across frames.
    // This means that, say, if a node has an input named "Pitch", then switches it next frame to "Volume", the same cables will persist the next frame, causing the user to get their speakers blown out, probably.
    // Instead, each input should have an Id<Input> or something that identifies it.
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

    /// "Extra" widget to put to the side of the input.
    pub extra: Option<Box<dyn FnOnce(&mut egui::Ui) + 'a>>,
}

impl Default for NodeInputUiOptions<'_> {
    fn default() -> Self {
        Self {
            display: &crate::widget::DefaultValueDisplay,
            display_range: Rangef::new(0.0, 1.0),
            range: Rangef::new(0.0, 1.0),
            base_drag_speed: None,
            default_value: 0.0,
            interactable: true,
            extra: None,
        }
    }
}

impl<'a> NodeInputUiOptions<'a> {
    pub fn uninteractable() -> Self {
        Self {
            interactable: false,
            ..Default::default()
        }
    }
    pub fn pitch() -> Self {
        pitch_internal(PitchState::Absolute)
    }
    pub fn pitch_relative() -> Self {
        pitch_internal(PitchState::Relative)
    }
    /// `pitch()` or `pitch_relative`, depending on a user-editable dropdown.
    pub fn pitch_choice(pitch_state: &'a mut PitchState) -> Self {
        let parent = pitch_internal(*pitch_state);
        Self {
            extra: Some(Box::new(|ui| {
                egui::ComboBox::from_id_salt(0)
                    .selected_text(&*pitch_state.short().encode_utf8(&mut [0u8; 4]))
                    .width(4.0)
                    .show_ui(ui, |ui| {
                        for &new_pitch in PitchState::ALL {
                            ui.selectable_value(pitch_state, new_pitch, new_pitch.long());
                        }
                    });
            })),
            ..parent
        }
    }
}

fn pitch_internal<'a>(pitch_state: PitchState) -> NodeInputUiOptions<'a> {
    struct PitchDisplay<const IS_RELATIVE: bool>;
    fn get_parts(val: f32) -> (String, String) {
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
    impl<const IS_RELATIVE: bool> PitchDisplay<IS_RELATIVE> {
        fn is_relative(&self, ctx: &ValueHandlerContext) -> bool {
            IS_RELATIVE || ctx.is_relative
        }
    }

    impl<const IS_RELATIVE: bool> ValueHandler for PitchDisplay<IS_RELATIVE> {
        fn to_input(&self, val: f32, ctx: &ValueHandlerContext) -> String {
            if self.is_relative(ctx) {
                format!("{:+.2}", val * 12.0)
            } else {
                let (note_name, difference) = get_parts(val);
                note_name + &difference
            }
        }
        fn parse_input(&self, str: &str, ctx: &ValueHandlerContext) -> Option<f32> {
            if let Ok(val) = str.parse::<f32>() {
                return Some(val / 12.0);
            }
            if self.is_relative(ctx) {
                return None;
            }

            let str = str.to_ascii_uppercase();
            let (note_name, difference) = str.split_once(['+', '-']).unwrap_or((&*str, ""));
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
        fn snap(&self, val: f32, ctx: &ValueHandlerContext) -> f32 {
            // relative controls by default don't snap, while nonrelative controls do snap.
            if ctx.alternate() ^ ctx.is_relative {
                val
            } else {
                (val * 12.0).round() / 12.0
            }
        }
    }
    NodeInputUiOptions {
        display: match pitch_state {
            PitchState::Absolute => &PitchDisplay::<false> as &dyn ValueHandler,
            PitchState::Relative => &PitchDisplay::<true> as &dyn ValueHandler,
        },
        display_range: Rangef::new(-2.0, 4.0),
        range: Rangef::EVERYTHING,

        ..Default::default()
    }
}

#[derive(
    Clone, Copy, PartialEq, Eq, zerocopy::TryFromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
#[repr(u8)]
pub enum PitchState {
    Relative = 0,
    Absolute = 1,
}
impl PitchState {
    pub const ALL: &[Self] = &[Self::Relative, Self::Absolute];
    pub fn short(self) -> char {
        match self {
            Self::Relative => 'R',
            Self::Absolute => 'A',
        }
    }
    pub fn long(self) -> &'static str {
        match self {
            Self::Relative => "Relative",
            Self::Absolute => "Absolute",
        }
    }
}
