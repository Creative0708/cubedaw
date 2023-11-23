use egui::{
    ecolor::rgb_from_hsv,
    epaint::{Hsva, Shadow},
    style::{Selection, WidgetVisuals, Widgets},
    Color32, FontData, FontDefinitions, FontFamily, Rgba, Rounding, Stroke, Visuals,
};

pub fn set_style(ctx: &egui::Context) {
    // Fonts
    let mut fonts = FontDefinitions::default();

    let base_font = FontData::from_static(include_bytes!("../../resources/Quantico-Regular.ttf"));

    fonts.font_data.insert("quantico".into(), base_font);

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "quantico".into());

    ctx.set_fonts(fonts);

    // Theme

    ctx.set_visuals(cubedaw_theme());
}

fn purpleify(factor: f32) -> Color32 {
    let rgb = rgb_from_hsv((0.59 + factor.powf(0.5) * 0.3, 1.0 - factor, 0.1 + factor));
    Color32::from_rgb(
        (rgb[0] * 255.0) as u8,
        (rgb[1] * 255.0) as u8,
        (rgb[2] * 255.0) as u8,
    )
}
fn purpleify_stroke(factor: f32, width: f32) -> Stroke {
    Stroke::new(width, purpleify(factor))
}

pub fn cubedaw_theme() -> Visuals {
    Visuals {
        dark_mode: true,
        override_text_color: None,
        widgets: cubedaw_widget_visuals(),
        selection: Selection::default(),
        // hyperlink_color: todo!(),
        faint_bg_color: purpleify(0.03).additive(),
        extreme_bg_color: purpleify(0.1),
        code_bg_color: purpleify(0.25),
        // warn_fg_color: todo!(),
        // error_fg_color: todo!(),
        window_rounding: Rounding::same(8.0),
        window_shadow: Shadow {
            extrusion: 12.0,
            color: purpleify(0.5).gamma_multiply(0.3).additive(),
        },
        window_fill: purpleify(0.25),
        window_stroke: purpleify_stroke(0.4, 1.0),
        menu_rounding: Rounding::ZERO,
        panel_fill: purpleify(0.25),
        // popup_shadow: todo!(),
        // resize_corner_size: todo!(),
        // text_cursor: todo!(),
        // text_cursor_preview: todo!(),
        // clip_rect_margin: todo!(),
        // button_frame: todo!(),
        // collapsing_header_frame: todo!(),
        // indent_has_left_vline: todo!(),
        // striped: todo!(),
        // slider_trailing_fill: todo!(),
        // interact_cursor: todo!(),
        // image_loading_spinners: todo!(),
        ..Visuals::dark()
    }
}

pub fn cubedaw_widget_visuals() -> Widgets {
    Widgets {
        noninteractive: WidgetVisuals {
            weak_bg_fill: purpleify(0.5),
            bg_fill: purpleify(0.5),
            bg_stroke: purpleify_stroke(0.3, 1.0),
            fg_stroke: purpleify_stroke(0.85, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            weak_bg_fill: purpleify(0.35),
            bg_fill: purpleify(0.35),
            bg_stroke: purpleify_stroke(0.55, 1.0),
            fg_stroke: purpleify_stroke(0.8, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            weak_bg_fill: purpleify(0.4),
            bg_fill: purpleify(0.4),
            bg_stroke: purpleify_stroke(0.5, 1.0),
            fg_stroke: purpleify_stroke(1.0, 1.5),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            weak_bg_fill: purpleify(0.17),
            bg_fill: purpleify(0.17),
            bg_stroke: purpleify_stroke(0.3, 1.0),
            fg_stroke: purpleify_stroke(0.8, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            weak_bg_fill: purpleify(0.1),
            bg_fill: purpleify(0.1),
            bg_stroke: purpleify_stroke(0.2, 1.0),
            fg_stroke: purpleify_stroke(0.8, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        ..Widgets::dark()
    }
}
