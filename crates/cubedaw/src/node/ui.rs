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
            fn to_input(&self, val: f32, ctx: &ValueHandlerContext) -> String {
                if ctx.is_relative {
                    format!("{:+.2}", val * 12.0)
                } else {
                    let (note_name, difference) = self.get_parts(val);
                    note_name + &difference
                }
            }
            fn parse_input(&self, str: &str, _ctx: &ValueHandlerContext) -> Option<f32> {
                if let Ok(val) = str.parse::<f32>() {
                    Some(val / 12.0)
                } else {
                    let str = str.to_ascii_uppercase();
                    let (note_name, difference) = match str.find(['+', '-']) {
                        Some(index) => str.split_at(index),
                        None => (&*str, ""),
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
            fn snap(&self, val: f32, ctx: &ValueHandlerContext) -> f32 {
                // relative controls by default don't snap, while nonrelative controls do snap.
                if ctx.alternate() ^ ctx.is_relative {
                    val
                } else {
                    (val * 12.0).round() / 12.0
                }
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