//! Approximation functions for f32x4 (modified functions from [micromath](https://github.com/tarcieri/micromath

// Some code is taken/modified from the micromath repository, which is dually licensed under the MIT and Apache licenses.
// https://github.com/tarcieri/micromath/tree/main?tab=readme-ov-file#license

use crate::wasm::{self, v128};

/// Sign mask.
pub(crate) const SIGN_MASK: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;

/// Exponent mask.
pub(crate) const EXPONENT_MASK: u32 = 0b0111_1111_1000_0000_0000_0000_0000_0000;

/// Mantissa mask.
pub(crate) const MANTISSA_MASK: u32 = 0b0000_0000_0111_1111_1111_1111_1111_1111;

/// Exponent mask.
pub(crate) const EXPONENT_BIAS: u32 = 127;

/// Mantissa bits.
///
/// Note: `MANTISSA_DIGITS` is available in `core::f32`, but the actual bits taken up are 24 - 1.
pub(crate) const MANTISSA_BITS: u32 = 23;

/// Extract exponent bits.
pub(crate) fn extract_exponent_bits(v: v128) -> v128 {
    wasm::u32x4_shr(
        wasm::v128_and(v, wasm::u32x4_splat(EXPONENT_MASK)),
        MANTISSA_BITS,
    )
}

/// Extract the exponent of a float's value.
pub(crate) fn extract_exponent_value(v: v128) -> v128 {
    wasm::i32x4_sub(
        extract_exponent_bits(v),
        wasm::i32x4_splat(EXPONENT_BIAS as i32),
    )
}

impl super::f32x16 {
    #[allow(clippy::excessive_precision)]
    pub fn ln(self) -> Self {
        // almost a direct translation of
        // https://github.com/tarcieri/micromath/blob/main/src/float/ln.rs

        fn ln_inner(val: v128) -> v128 {
            let base2_exponent = extract_exponent_value(val);
            let divisor = wasm::v128_and(val, wasm::u32x4_splat(EXPONENT_MASK));
            let x_working = wasm::f32x4_div(val, divisor);

            // i think we need more wasm::f32x4
            let ln_1to2_polynomial = wasm::f32x4_add(
                wasm::f32x4_splat(-1.741_793_9),
                wasm::f32x4_mul(
                    wasm::f32x4_add(
                        wasm::f32x4_splat(2.821_202_6),
                        wasm::f32x4_mul(
                            wasm::f32x4_add(
                                wasm::f32x4_splat(-1.469_956_8),
                                wasm::f32x4_mul(
                                    wasm::f32x4_sub(
                                        wasm::f32x4_splat(0.447_179_55),
                                        wasm::f32x4_mul(
                                            wasm::f32x4_splat(0.056_570_851),
                                            x_working,
                                        ),
                                    ),
                                    x_working,
                                ),
                            ),
                            x_working,
                        ),
                    ),
                    x_working,
                ),
            );

            wasm::f32x4_add(
                wasm::f32x4_mul(
                    wasm::f32x4_convert_i32x4(base2_exponent),
                    wasm::f32x4_splat(core::f32::consts::LN_2),
                ),
                ln_1to2_polynomial,
            )
        }

        super::every!(ln_inner, self)
    }
    pub fn exp(self) -> Self {
        // https://github.com/tarcieri/micromath/blob/main/src/float/exp.rs

        fn exp_inner(val: v128) -> v128 {
            let x_ln2recip = wasm::f32x4_mul(val, wasm::f32x4_splat(core::f32::consts::LOG2_E));
            let x_trunc = wasm::f32x4_trunc(x_ln2recip);
            let x_fract = wasm::f32x4_sub(x_ln2recip, x_trunc);

            let x_fract = wasm::f32x4_mul(x_fract, wasm::f32x4_splat(core::f32::consts::LN_2));

            let fract_exp = {
                let mut total = wasm::f32x4_splat(1.0);
                for i in (1..=4).rev() {
                    total = wasm::f32x4_add(
                        wasm::f32x4_splat(1.0),
                        wasm::f32x4_mul(
                            x_fract,
                            wasm::f32x4_div(wasm::f32x4_splat(1.0), wasm::f32x4_splat(i as f32)),
                        ),
                    );
                }
                total
            };

            wasm::i32x4_add(
                fract_exp,
                wasm::i32x4_shl(wasm::i32x4_relaxed_trunc_f32x4(x_trunc), MANTISSA_BITS),
            )
        }

        super::every!(exp_inner, self)
    }

    pub fn powf(self, other: Self) -> Self {
        (self.ln() * other).exp()
    }

    pub fn sin(self) -> Self {
        (self - Self::splat(core::f32::consts::FRAC_PI_2)).cos()
    }
    pub fn cos(self) -> Self {
        // https://github.com/tarcieri/micromath/blob/main/src/float/cos.rs
        let mut x = self;
        x *= Self::splat(core::f32::consts::FRAC_1_PI * 0.5);

        x -= Self::splat(0.25) + (x + Self::splat(0.25)).floor();
        x *= Self::splat(16.0) * (x.abs() - Self::splat(0.5));
        x += Self::splat(0.225) * x * (x.abs() - Self::splat(1.0));

        x
    }
}

#[cfg(test)]
mod tests {
    use crate::f32x16;

    pub(crate) const MAX_ERROR: f32 = 0.001;

    /// ln(x) test vectors - `(input, output)`
    pub(crate) const LN_TEST_VECTORS: &[(f32, f32)] = &[
        (1e-20, -46.0517),
        (1e-19, -43.749115),
        (1e-18, -41.446533),
        (1e-17, -39.143948),
        (1e-16, -36.841362),
        (1e-15, -34.538776),
        (1e-14, -32.23619),
        (1e-13, -29.933607),
        (1e-12, -27.631021),
        (1e-11, -25.328436),
        (1e-10, -23.02585),
        (1e-09, -20.723267),
        (1e-08, -18.420681),
        (1e-07, -16.118095),
        (1e-06, -13.815511),
        (1e-05, -11.512925),
        (1e-04, -9.2103405),
        (0.001, -6.9077554),
        (0.01, -4.6051702),
        (0.1, -2.3025851),
        (10.0, 2.3025851),
        (100.0, 4.6051702),
        (1000.0, 6.9077554),
        (10000.0, 9.2103405),
        (100000.0, 11.512925),
        (1000000.0, 13.815511),
        (10000000.0, 16.118095),
        (100000000.0, 18.420681),
        (1000000000.0, 20.723267),
        (10000000000.0, 23.02585),
        (100000000000.0, 25.328436),
        (1000000000000.0, 27.631021),
        (10000000000000.0, 29.933607),
        (100000000000000.0, 32.23619),
        (1000000000000000.0, 34.538776),
        (1e+16, 36.841362),
        (1e+17, 39.143948),
        (1e+18, 41.446533),
        (1e+19, 43.749115),
    ];

    #[test]
    fn sanity_check_ln() {
        assert_eq!(f32x16::splat(1.0).ln(), f32x16::splat(0.0));

        for &(x, expected) in LN_TEST_VECTORS {
            let ln_x = f32x16::splat(x).ln();
            let res = ln_x - f32x16::splat(expected);
            assert_eq!(
                f32x16::splat(res.extract(0)),
                res,
                "ln doesn't work with all lanes"
            );

            let relative_error = res.extract(0) / expected;

            assert!(
                relative_error <= MAX_ERROR,
                "relative_error {} too large: {} vs {}",
                relative_error,
                ln_x.extract(0),
                expected
            );
        }
    }
    #[test]
    fn sanity_check_exp() {
        for (x, expected) in LN_TEST_VECTORS.iter().map(|&(a, b)| (b, a)) {
            let exp_x = f32x16::splat(x).exp();
            let res = exp_x - f32x16::splat(expected);
            assert_eq!(
                f32x16::splat(res.extract(0)),
                res,
                "exp doesn't work with all lanes"
            );

            let relative_error = res.extract(0) / expected;

            assert!(
                relative_error <= MAX_ERROR,
                "relative_error {} too large: {} vs {}",
                relative_error,
                exp_x.extract(0),
                expected
            );
        }
    }

    /// Cosine test vectors - `(input, output)`
    #[allow(clippy::approx_constant)]
    const TEST_VECTORS_COS: &[(f32, f32)] = &[
        (0.000, 1.000),
        (0.140, 0.990),
        (0.279, 0.961),
        (0.419, 0.914),
        (0.559, 0.848),
        (0.698, 0.766),
        (0.838, 0.669),
        (0.977, 0.559),
        (1.117, 0.438),
        (1.257, 0.309),
        (1.396, 0.174),
        (1.536, 0.035),
        (1.676, -0.105),
        (1.815, -0.242),
        (1.955, -0.375),
        (2.094, -0.500),
        (2.234, -0.616),
        (2.374, -0.719),
        (2.513, -0.809),
        (2.653, -0.883),
        (2.793, -0.940),
        (2.932, -0.978),
        (3.072, -0.998),
        (3.211, -0.998),
        (3.351, -0.978),
        (3.491, -0.940),
        (3.630, -0.883),
        (3.770, -0.809),
        (3.910, -0.719),
        (4.049, -0.616),
        (4.189, -0.500),
        (4.328, -0.375),
        (4.468, -0.242),
        (4.608, -0.105),
        (4.747, 0.035),
        (4.887, 0.174),
        (5.027, 0.309),
        (5.166, 0.438),
        (5.306, 0.559),
        (5.445, 0.669),
        (5.585, 0.766),
        (5.725, 0.848),
        (5.864, 0.914),
        (6.004, 0.961),
        (6.144, 0.990),
        (6.283, 1.000),
    ];

    #[test]
    fn sanity_check_cos() {
        for &(x, expected) in TEST_VECTORS_COS {
            let cos_x = f32x16::splat(x).cos();
            let res = cos_x - f32x16::splat(expected);
            assert_eq!(
                f32x16::splat(res.extract(0)),
                res,
                "ln doesn't work with all lanes"
            );

            let delta = (cos_x.extract(0) - expected).abs();

            assert!(
                delta <= MAX_ERROR,
                "delta {} too large: {} vs {}",
                delta,
                cos_x.extract(0),
                expected
            );
        }
    }
}
