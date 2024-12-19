pub mod builtin;

mod math;
pub use math::MathNode;
mod oscillator;
pub use oscillator::OscillatorNode;

trait ZerocopyTryFromExt {
    type Output;
    fn anyhow(self) -> Self::Output;
}
impl<'a, T> ZerocopyTryFromExt for Result<(&'a T, &'a [u8]), zerocopy::TryCastError<&'a [u8], T>>
where
    T: zerocopy::TryFromBytes,
{
    type Output = anyhow::Result<(&'a T, &'a [u8])>;
    fn anyhow(self) -> Self::Output {
        self.map_err(|err| {
            anyhow::anyhow!(
                "zerocopy::TryFromBytes::*() failed for {}",
                core::any::type_name::<T>()
            )
        })
    }
}
impl<'a, T> ZerocopyTryFromExt
    for Result<(&'a mut T, &'a mut [u8]), zerocopy::TryCastError<&'a mut [u8], T>>
where
    T: zerocopy::TryFromBytes,
{
    type Output = anyhow::Result<(&'a mut T, &'a mut [u8])>;
    fn anyhow(self) -> Self::Output {
        self.map_err(|err| {
            anyhow::anyhow!(
                "zerocopy::TryFromBytes::*() failed for {}",
                core::any::type_name::<T>()
            )
        })
    }
}
