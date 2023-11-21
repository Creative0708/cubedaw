use egui::Rect;

/// Takes a rect and interpolates it into another rect.
pub fn subrect(r1: Rect, r2: Rect) -> Rect {
    Rect {
        min: r2.lerp_inside(r1.min.to_vec2()),
        max: r2.lerp_inside(r1.max.to_vec2()),
    }
}
