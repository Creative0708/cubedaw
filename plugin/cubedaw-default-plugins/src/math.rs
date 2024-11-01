use core::f32::consts;

use cubedaw_pluginlib::f32x16;

const TABLE_SIZE: usize = 256;
static SINE_TABLE: [f32; TABLE_SIZE + 2] = {
    /// Very slow but accurate sine function using the Taylor series.
    /// Good enough for a lookup table at compile time.
    const fn slow_const_sin(x: f32) -> f32 {
        let mut i = 0;

        let mut val = 1.0;
        let mut ans = 0.0;
        loop {
            i += 1;
            val *= x / i as f32;
            if i % 2 == 1 {
                let prev_ans = ans;
                if i % 4 == 1 {
                    ans += val;
                } else {
                    ans -= val;
                }
                if prev_ans == ans {
                    // val is too small to contribute anything to ans anymore, stop the loop
                    break;
                }
            }
        }
        ans
    }

    let mut table = [0.0; TABLE_SIZE + 2];
    let mut i = 0;
    while i < table.len() {
        table[i] = slow_const_sin(i as f32 / TABLE_SIZE as f32 * consts::TAU);
        i += 1;
    }
    table
};

/// Computes sin(x * tau) with a lookup table. Very fast.
///
/// # Safety
/// If `0.0 <= x <= 1.0` is true, then this function is safe. Everything else is UB.
/// This makes infinities and NaNs UB as well. So make sure that doesn't happen.
pub unsafe fn sin01_unchecked(x: f32) -> f32 {
    let val = x * TABLE_SIZE as f32;

    // SAFETY: 0.0 <= x <= 1.0, so 0.0 <= val <= TABLE_SIZE. TABLE_SIZE + 2 is a usize, so this can't overflow.
    let index: usize = unsafe { val.to_int_unchecked() };
    let fraction = val - index as f32;

    // SAFETY: 0 <= index <= TABLE_SIZE
    let (val1, val2) = unsafe {
        (
            *SINE_TABLE.get_unchecked(index),
            *SINE_TABLE.get_unchecked(index + 1),
        )
    };

    (val2 - val1) * fraction + val1
}

/// Computes sin(x * tau) with a lookup table. Kinda fast.
pub fn sin01(x: f32) -> f32 {
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

const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
const MULT_PER_PITCH: f32 = 1.0594631f32; // 2**(1/12)

pub fn pitch_to_hertz_simd(pitch: f32x16) -> f32x16 {
    const MIDDLE_C_FREQUENCY: f32 = 261.62558f32; // 440 / 2**(9/12)
    const MULT_PER_PITCH: f32 = 1.0594631f32; // 2**(1/12)

    f32x16::splat(MIDDLE_C_FREQUENCY) * f32x16::splat(MULT_PER_PITCH).powf(pitch)
}

#[cfg(test)]
mod tests {
    use crate::math::sin01_unchecked;

    use super::sin01;

    #[test]
    fn test_sin01_unchecked() {
        unsafe fn test_about_eq(x: f32) {
            let expected = (x * core::f32::consts::TAU).sin();
            let actual = unsafe { sin01_unchecked(x) };
            if (actual - expected).abs() > 1e-5 {
                panic!(
                    "sin01_unchecked({:.02}) failed: expected {expected:.05}, got {actual:.05}",
                    x * core::f32::consts::TAU
                );
            }
        }

        unsafe {
            for i in 0..=40 {
                test_about_eq(i as f32 / 40.0);
            }
        }
    }

    #[test]
    fn test_sin01() {
        unsafe fn test_about_eq(x: f32) {
            let expected = (x * core::f32::consts::TAU).sin();
            let actual = unsafe { sin01_unchecked(x) };
            if (actual - expected).abs() > 1e-5 {
                panic!(
                    "sin01({:.02}) failed: expected {expected:.05}, got {actual:.05}",
                    x * core::f32::consts::TAU
                );
            }
        }

        unsafe {
            for i in 0..=40 {
                test_about_eq(i as f32 / 11.0);
            }
        }
    }
}
