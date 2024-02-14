use egui::{
    ecolor::rgb_from_hsv,
    epaint::Shadow,
    style::{Selection, WidgetVisuals, Widgets},
    Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Visuals,
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

fn theme(factor: f32) -> Color32 {
    let rgb = rgb_from_hsv((
        0.75 - factor.powf(0.5) * 0.1,
        f32::min((1.0 - factor).powf(2.0), 1.0),
        f32::min(factor * factor * 2.0 + 0.05, 1.0),
    ));
    Color32::from_rgb(
        (rgb[0] * 255.0) as u8,
        (rgb[1] * 255.0) as u8,
        (rgb[2] * 255.0) as u8,
    )
}
fn theme_stroke(factor: f32, width: f32) -> Stroke {
    Stroke::new(width, theme(factor))
}

pub fn cubedaw_theme() -> Visuals {
    use bytemuck::must_cast as transmute;
    Visuals {
        dark_mode: true,
        override_text_color: None,
        widgets: cubedaw_widget_visuals(),
        selection: Selection::default(),
        // hyperlink_color: todo!(),
        faint_bg_color: transmute::<u32, Color32>(
            transmute::<Color32, u32>(theme(0.2)) - transmute::<Color32, u32>(theme(0.1)),
        )
        .additive(),
        extreme_bg_color: theme(0.2),
        code_bg_color: theme(0.2),
        warn_fg_color: Color32::from_rgb(255, 238, 0),
        error_fg_color: Color32::from_rgb(255, 119, 0),
        window_rounding: Rounding::same(8.0),
        window_shadow: Shadow {
            extrusion: 12.0,
            color: theme(0.5).gamma_multiply(0.3).additive(),
        },
        window_fill: theme(0.3),
        window_stroke: theme_stroke(0.4, 1.0),
        // menu_rounding: Rounding::ZERO,
        panel_fill: theme(0.35),
        popup_shadow: Shadow {
            extrusion: 6.0,
            color: theme(0.5).gamma_multiply(0.2).additive(),
        },
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
            weak_bg_fill: theme(0.5),
            bg_fill: theme(0.5),
            bg_stroke: theme_stroke(0.3, 1.0),
            fg_stroke: theme_stroke(0.85, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        inactive: WidgetVisuals {
            weak_bg_fill: theme(0.4),
            bg_fill: theme(0.4),
            bg_stroke: theme_stroke(0.5, 1.0),
            fg_stroke: theme_stroke(0.8, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 0.0,
        },
        hovered: WidgetVisuals {
            weak_bg_fill: theme(0.45),
            bg_fill: theme(0.45),
            bg_stroke: theme_stroke(0.55, 1.0),
            fg_stroke: theme_stroke(0.85, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        active: WidgetVisuals {
            weak_bg_fill: theme(0.4),
            bg_fill: theme(0.4),
            bg_stroke: theme_stroke(0.6, 1.0),
            fg_stroke: theme_stroke(1.0, 1.5),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        open: WidgetVisuals {
            weak_bg_fill: theme(0.3),
            bg_fill: theme(0.3),
            bg_stroke: theme_stroke(0.2, 1.0),
            fg_stroke: theme_stroke(0.8, 1.0),
            rounding: Rounding::same(2.0),
            expansion: 1.0,
        },
        ..Widgets::dark()
    }
}
