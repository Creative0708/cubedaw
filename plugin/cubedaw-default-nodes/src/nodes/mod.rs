mod math;
mod distortion
mod oscillator;

#[derive(
    Clone, Copy, PartialEq, Eq, zerocopy::TryFromBytes, zerocopy::IntoBytes, zerocopy::Immutable,
)]
#[allow(dead_code)]
#[repr(u8)]
pub enum PitchState {
    Relative = 0,
    Absolute = 1,
}
impl PitchState {
    pub fn is_relative(self) -> bool {
        matches!(self, Self::Relative)
    }
}
