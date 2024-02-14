use egui::Rect;

/// Takes a rect and interpolates it into another rect.
pub fn subrect(r1: Rect, r2: Rect) -> Rect {
    Rect {
        min: r2.lerp_inside(r1.min.to_vec2()),
        max: r2.lerp_inside(r1.max.to_vec2()),
    }
}

pub fn frange(start: f32, end: f32, step: f32) -> impl Iterator<Item = f32> {
    return (0..)
        .map(move |x| start + x as f32 * step)
        .take_while(move |&x| x < end);
}

pub fn frange_viewport(
    height: f32,
    clip_start: f32,
    clip_end: f32,
) -> impl Iterator<Item = (u32, f32)> {
    return ((clip_start / height) as u32..)
        .map(move |x| (x, x as f32 * height))
        .take_while(move |&(_, x)| x < clip_end);
}
