// fn fast_sin(x: f32) -> f32 {
//     use std::f32::consts;
//     use std::sync::LazyLock;

//     const TABLE_SIZE: usize = 1024;
//     static SINE_TABLE: LazyLock<[f32; TABLE_SIZE]> = LazyLock::new(|| {
//         let mut table = [0.0; TABLE_SIZE];
//         for (i, val) in table.iter_mut().enumerate() {
//             *val = f32::sin(i as f32 / TABLE_SIZE as f32 * consts::FRAC_PI_2);
//         }
//         table
//     });
//     let normalized = x * consts::FRAC_2_PI;
//     let y = normalized.copysign(1.0);

//     // The u32 representation of positive finite f32s is monotonically increasing
//     // (that is, if y.to_bits() > x.to_bits(), y > x and vice versa)
//     // Additionally, the NaNs and infinities have an exponent field of all ones, so
//     // they are all always greater than every finite positive f32.
//     // This means we can do a cheap u32 comparison to check if a value either
//     // causes UB in to_int_unchecked or is too imprecise to produce a useful result.
//     // In both cases 0 is a sensible return value.
//     //
//     // Yippee!
//     let casted = y.to_bits();
//     // 67108864.0f32. Beyond this the f32 is too imprecise to be
//     // anything other than an integer that is 0 (mod 4).
//     if casted >= 0b01001100100000000000000000000000 {
//         return 0.0;
//     }

//     // SAFETY: We checked for NaNs, infinities, and representability.
//     // See above for reasoning.
//     let int = unsafe { y.to_int_unchecked::<i32>() };

//     let fract = y - int as f32;
//     let flip_y = normalized.is_sign_negative() ^ (int & 2 != 0);
//     let flip_x = int & 1 != 0;

//     let index = fract

//     SINE_TABLE[if flip_x {}]
// }

use cubedaw_lib::NodeInputUiOptions;
use egui::ComboBox;

/// `sin(x * TAU)`, but faster with a lookup table.
///
/// SAFETY: `0.0 <= x <= 1.0`. Obviously x can't be infinite or NaN.
unsafe fn sin01(mut x: f32) -> f32 {
    debug_assert!(
        (0.0..=1.0).contains(&x),
        "{x} is not in the range 0.0..=1.0"
    );
    use std::f32::consts;
    use std::sync::LazyLock;

    const TABLE_SIZE: usize = 1 << 10;
    // + 1 to not oob on last
    static SINE_TABLE: LazyLock<[f32; TABLE_SIZE + 1]> = LazyLock::new(|| {
        let mut table = [0.0; TABLE_SIZE + 1];
        for (i, val) in table.iter_mut().enumerate() {
            *val = f32::sin(i as f32 / TABLE_SIZE as f32 * consts::FRAC_PI_2);
        }
        table
    });

    let times_4 = x * 4.0;
    // SAFETY: since 0.0 <= x <= 1.0, 0.0 <= times_4 <= 4.0. Last I checked, 4 fits in an i32. I think.
    let int_times_4 = unsafe { times_4.to_int_unchecked::<i32>() };

    let flip_x = int_times_4 & 1 != 0;
    let flip_y = int_times_4 & 2 != 0;

    if flip_x {
        x = 1.0 - x;
    }
    // 0.0 <= index <= TABLE_SIZE * 4
    let index = x * (TABLE_SIZE * 4) as f32;
    let trunc = index.trunc();
    let ratio = index - trunc;

    const _: () = assert!(TABLE_SIZE * 4 <= i32::MAX as usize);

    // SAFETY: 0.0 <= index <= TABLE_SIZE * 4. TABLE_SIZE * 4 fits in an i32 so this is safe
    // 0 <= i1 <= TABLE_SIZE - 1
    let i1 = unsafe { index.to_int_unchecked::<i32>() } as usize;
    // 1 <= i2 <= TABLE_SIZE. Note that SINE_TABLE.len() == TABLE_SIZE
    let i2 = i1 + 1;

    // linearly interpolate between the values
    let mut res = SINE_TABLE[i1] * (1.0 - ratio) + SINE_TABLE[i2] * ratio;
    if flip_y {
        res = res.copysign(-1.0);
    }
    res
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sin01() {
        use crate::node::oscillator::sin01;

        unsafe fn test_about_eq(x: f32) {
            let expected = (x * std::f32::consts::TAU).sin();
            let actual = unsafe { sin01(x) };
            if (actual - expected).abs() > f32::EPSILON {
                panic!(
                    "sin({:.02}) failed: expected {expected:.02}, got {actual:.02}",
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
        let mut output = ctx.output(0);
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
