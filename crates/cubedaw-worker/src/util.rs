/// [Linear interpolation](https://en.wikipedia.org/wiki/Linear_interpolation).
pub fn lerp(from: f32, to: f32, alpha: f32) -> f32 {
    from * (1.0 - alpha) + to * alpha
}

#[cfg(test)]
mod tests {
    use std::f32::consts;

    #[test]
    fn test_lerp() {
        fn testcase(from: f32, to: f32, alpha: f32, expected: f32) {
            let lerped = super::lerp(from, to, alpha);
            assert!(
                (lerped - expected).abs() < f32::EPSILON,
                "lerp({from}, {to}, {alpha}) failed; expected {expected}, got {lerped}"
            );
        }
        testcase(50.0, 100.0, 0.5, 75.0);
        testcase(0.0, consts::PI, 1.0 / 3.0, consts::FRAC_PI_3);
        testcase(10000.0, -10000.0, 0.99, -9800.0);
    }
}
