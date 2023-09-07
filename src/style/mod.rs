use egui::{Visuals, Color32, style::{Widgets, WidgetVisuals, Selection}, Stroke, Rounding};
use egui::epaint::Shadow;

pub fn get_visuals() -> Visuals {
    let defaul_color = Color32::BLACK;
    let default_stroke = Stroke {
        width: 1.0,
        color: Color32::BLACK
    };
    let default_widget = WidgetVisuals {
        bg_fill: Color32::WHITE,
        bg_stroke: default_stroke,
        fg_stroke: default_stroke,
        rounding: Rounding::default(),
        expansion: 0.0
    };
    Visuals { 
        dark_mode: false, 
        override_text_color: None, 
        widgets: Widgets { 
            noninteractive: default_widget, 
            inactive: default_widget, 
            hovered: default_widget, 
            active: default_widget, 
            open: default_widget
        }, 
        selection: Selection {
            bg_fill: Color32::WHITE,
            stroke: default_stroke
        }, 
        hyperlink_color: defaul_color, 
        faint_bg_color: defaul_color, 
        extreme_bg_color: defaul_color, 
        code_bg_color: defaul_color, 
        warn_fg_color: defaul_color, 
        error_fg_color: defaul_color, 
        window_rounding: Rounding::default(), 
        window_shadow: Shadow::default(), 
        popup_shadow: Shadow::default(),
        resize_corner_size: 1.0, 
        text_cursor_width: 1.0, 
        text_cursor_preview: true, 
        clip_rect_margin: 1.0, 
        button_frame: true, 
        collapsing_header_frame: false 
    }
}

