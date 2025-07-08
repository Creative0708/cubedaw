use std::iter;

pub fn frange(start: f32, end: f32, step: f32) -> impl Iterator<Item = f32> {
    let mut remaining = Some(((end - start) / step) as usize);

    let mut curr = start;
    iter::from_fn(move || {
        remaining = remaining?.checked_sub(1);
        let orig = curr;
        curr += step;
        Some(orig)
    })
}
pub fn frange_snapped(start: f32, end: f32, step: f32) -> impl Iterator<Item = f32> {
    frange((start / step).ceil() * step, end, step)
}
