#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
    zerocopy::TryFromBytes,
    zerocopy::IntoBytes,
    zerocopy::Immutable,
)]
#[allow(dead_code)]
#[repr(u8)]
pub enum OscillatorNodeType {
    Sine = 0,
    Saw = 1,
    Square = 2,
    Triangle = 3,
}
#[repr(C)]
#[derive(
    zerocopy::TryFromBytes, zerocopy::IntoBytes, zerocopy::Immutable, zerocopy::KnownLayout,
)]
pub struct OscillatorNodeArgs {
    pub node_type: OscillatorNodeType,
    pub pitch_state: super::PitchState,
    pub _pad1: [u8; 2],
}
