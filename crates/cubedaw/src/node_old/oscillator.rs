use cubedaw_lib::NodeInputUiOptions;
use egui::ComboBox;

#[derive(Debug, Clone)]
pub struct OscillatorNode {
    oscillator_cycle: f32,
}

impl cubedaw_lib::Node for OscillatorNode {
    type State = OscillatorNodeState;

    fn new() -> Self {
        Self {
            oscillator_cycle: 0.0,
        }
    }

    fn new_state(ctx: cubedaw_lib::NodeCreationContext<'_>) -> Self::State {
        OscillatorNodeState {
            node_type: match ctx.alias.as_deref() {
                Some("sine") => OscillatorNodeType::Sine,
                Some("saw") => OscillatorNodeType::Saw,
                Some("square") => OscillatorNodeType::Square,
                Some("triangle") => OscillatorNodeType::Triangle,

                _ => OscillatorNodeType::Sine,
            },
        }
    }

    // TODO optimize (saw/square/whatever waves are optimizable but how the hell do we optimize the sin lookup table)
    fn process(&mut self, state: &Self::State, ctx: &mut dyn cubedaw_lib::NodeContext<'_>) {
        let pitch = ctx.input(0);
        let output = ctx.output(0);
        for i in 0..ctx.buffer_size() {
            let oscillator_cycle = self.oscillator_cycle;
            let val = match state.node_type {
                OscillatorNodeType::Sine => unsafe {
                    // SAFETY: oscillator_cycle is always within the range 0.0..1.0.
                    sin01_unchecked(oscillator_cycle)
                },
                OscillatorNodeType::Saw => oscillator_cycle * 2.0 - 1.0,
                OscillatorNodeType::Square => 1.0f32.copysign(oscillator_cycle - 0.5),
                OscillatorNodeType::Triangle => 1.0 - (2.0 - oscillator_cycle * 4.0).abs(),
            };
            output.set(i, val);

            let increment = cubedaw_lib::pitch_to_hertz(pitch[i]) / ctx.sample_rate() as f32;
            // for IEEE 754 reasons all the infinities, NaNs, and negative numbers have a bit
            // representation that is greater than 1.0f32.to_bits().
            // this prevents a NaN/infinity from poisoning self.oscillator_cycle
            if increment.to_bits() < 1.0f32.to_bits() {
                self.oscillator_cycle += increment;
                if self.oscillator_cycle >= 1.0 {
                    self.oscillator_cycle -= 1.0;
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OscillatorNodeState {
    node_type: OscillatorNodeType,
}

impl cubedaw_lib::NodeState for OscillatorNodeState {
    fn title(&self) -> std::borrow::Cow<'_, str> {
        format!("Oscillator - {}", self.node_type.to_str()).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut dyn cubedaw_lib::NodeUiContext) {
        ctx.output_ui(ui, "Osc");

        ComboBox::from_id_source(0)
            .selected_text(self.node_type.to_str())
            .show_ui(ui, |ui| {
                for node_type in [
                    OscillatorNodeType::Sine,
                    OscillatorNodeType::Saw,
                    OscillatorNodeType::Square,
                    OscillatorNodeType::Triangle,
                ] {
                    ui.selectable_value(&mut self.node_type, node_type, node_type.to_str());
                }
            });

        // TODO implement plot

        ctx.input_ui(ui, "Pitch", NodeInputUiOptions::pitch());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OscillatorNodeType {
    Sine,
    Saw,
    Square,
    Triangle,
}

impl OscillatorNodeType {
    const fn to_str(self) -> &'static str {
        match self {
            Self::Sine => "Sine",
            Self::Saw => "Saw",
            Self::Square => "Square",
            Self::Triangle => "Triangle",
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sanity_check() {
        use std::f32::consts::*;

        for x in 0..=50 {
            let val = x as f32 / 50.0;
            let fast = unsafe { super::sin01_unchecked(val) };
            let accurate = (val * TAU).sin();
            assert!(
                (fast - accurate).abs() < 0.0001,
                "sin({}): expected {}, got {}",
                val * TAU,
                accurate,
                fast
            );
        }
    }
}
