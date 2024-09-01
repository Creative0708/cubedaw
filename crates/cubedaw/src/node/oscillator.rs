unsafe fn sin01(x: f32) -> f32 {
    use std::f32::consts;
    use std::sync::LazyLock;

    const TABLE_SIZE: usize = 1024;
    static SINE_TABLE: LazyLock<[f32; TABLE_SIZE + 2]> = LazyLock::new(|| {
        let mut table = [0.0; TABLE_SIZE + 2];
        for (i, val) in table.iter_mut().enumerate() {
            *val = f32::sin(i as f32 / TABLE_SIZE as f32 * consts::FRAC_PI_2);
        }
        table
    });

    let y = x * 4.0;

    // The u32 representation of positive finite f32s is monotonically increasing
    // (that is, if y.to_bits() > x.to_bits(), y > x and vice versa)
    // Additionally, the NaNs and infinities have an exponent field of all ones, so
    // they are all always greater than every finite positive f32.
    // This means we can do a cheap u32 comparison to check if a value either
    // causes UB in to_int_unchecked or is too imprecise to produce a useful result.
    // In both cases 0 is a sensible return value.
    //
    // Yippee!
    let casted = y.to_bits();

    // 67108864.0f32. Beyond this the f32 is too imprecise to be
    // anything other than an integer that is 0 (mod 4).
    if casted >= 0b01001100100000000000000000000000 {
        return 0.0;
    }

    // SAFETY: We checked for NaNs, infinities, and representability.
    // See above for reasoning.
    let int = unsafe { y.to_int_unchecked::<i32>() };

    let mut fract = y - int as f32;
    let flip_y = int & 2 != 0;
    let flip_x = int & 1 != 0;
    if flip_x {
        fract = 1.0 - fract;
    }

    let index_f32 = fract * TABLE_SIZE as f32;
    // SAFETY: fract is in the range 0.0..=1.0 so index_f32 is in the range of 0..=TABLE_SIZE
    let index = unsafe { index_f32.to_int_unchecked::<usize>() };

    // SAFETY: fract is in the range 0.0..=1.0 so index is in the range of 0..=TABLE_SIZE
    let (val1, val2) = unsafe {
        (
            *SINE_TABLE.get_unchecked(index),
            *SINE_TABLE.get_unchecked(index + 1),
        )
    };
    let mut val = val1 + (val2 - val1) * (index_f32 - index as f32);
    if flip_y {
        val = -val
    }
    val
}

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
        let volume = ctx.input(1);
        let output = ctx.output(0);
        for i in 0..ctx.buffer_size() {
            let oscillator_cycle = self.oscillator_cycle;
            let val = match state.node_type {
                OscillatorNodeType::Sine => unsafe {
                    // SAFETY: oscillator_cycle is always within the range 0.0..1.0.
                    sin01(oscillator_cycle)
                },
                OscillatorNodeType::Saw => oscillator_cycle * 2.0 - 1.0,
                OscillatorNodeType::Square => 1.0f32.abs(),
                OscillatorNodeType::Triangle => 1.0 - (2.0 - oscillator_cycle * 4.0).abs(),
            };
            output.set(i, val * volume[i]);

            let increment = cubedaw_lib::pitch_to_hertz(pitch[i]) / ctx.sample_rate() as f32;
            // for math reasons all the infinities, NaNs, and negative numbers have a bit
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
    fn test_sin01() {
        use crate::node::oscillator::sin01;

        unsafe fn test_about_eq(x: f32) {
            let expected = (x * std::f32::consts::TAU).sin();
            let actual = unsafe { sin01(x) };
            if (actual - expected).abs() > 1e-5 {
                panic!(
                    "sin({:.02}) failed: expected {expected:.05}, got {actual:.05}",
                    x * std::f32::consts::TAU
                );
            }
        }

        unsafe {
            for i in 0..=40 {
                test_about_eq(i as f32 / 40.0);
            }
            test_about_eq(1.0f32.next_down());
            test_about_eq(0.0f32.next_up());
        }
    }
}
