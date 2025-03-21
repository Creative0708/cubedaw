use cubedaw_pluginlib::{f32x16, Attribute};

use super::PitchState;

mod schema;
use schema::*;

#[derive(Debug)]
#[repr(C)]
pub struct OscillatorNodeState {
    cycle: f32,
}

#[no_mangle]
fn do_oscillator(state: &OscillatorNodeArgs, buf: &mut OscillatorNodeState) {
    let mut pitch = cubedaw_pluginlib::input::<0>();
    if state.pitch_state.is_relative() {
        pitch += cubedaw_pluginlib::attribute::<{ Attribute::Pitch }>()
    }

    let increment = crate::util::pitch_to_hertz(pitch)
        * f32x16::splat(1.0 / cubedaw_pluginlib::sample_rate() as f32);

    let cycle = increment.prefix_sum_with(buf.cycle).fract();
    buf.cycle = cycle.extract(15);

    let val = match state.node_type {
        OscillatorNodeType::Sine => (cycle * f32x16::splat(core::f32::consts::TAU)).sin(),
        OscillatorNodeType::Saw => cycle * f32x16::splat(2.0) - f32x16::splat(1.0),
        OscillatorNodeType::Square => f32x16::splat(1.0).copysign(cycle - f32x16::splat(0.5)),
        OscillatorNodeType::Triangle => {
            f32x16::splat(1.0) - (f32x16::splat(2.0) - cycle * f32x16::splat(4.0)).abs()
        }
    };

    cubedaw_pluginlib::output::<0>(val);
}

cubedaw_pluginlib::export_node!("cubedaw:oscillator", do_oscillator);
