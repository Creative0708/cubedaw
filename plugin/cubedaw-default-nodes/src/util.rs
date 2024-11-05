use cubedaw_pluginlib::f32x16;

pub fn pitch_to_hertz(pitch: f32x16) -> f32x16 {
    const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
    const MULT_PER_PITCH: f32 = 1.0594631f32; // 2**(1/12)

    f32x16::splat(MIDDLE_C_FREQUENCY) * f32x16::splat(MULT_PER_PITCH).powf(pitch)
}
