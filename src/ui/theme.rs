use egui::{Color32, Context, CornerRadius, Stroke, Style, Visuals};

pub const WINDOW: Color32 = Color32::from_rgb(22, 23, 25);
pub const SIDEBAR: Color32 = Color32::from_rgb(27, 29, 32);
pub const SURFACE: Color32 = Color32::from_rgb(32, 34, 38);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(41, 44, 49);
pub const BORDER: Color32 = Color32::from_rgb(52, 56, 62);
pub const TEXT: Color32 = Color32::from_rgb(240, 241, 242);
pub const SECONDARY: Color32 = Color32::from_rgb(165, 171, 179);
pub const MUTED: Color32 = Color32::from_rgb(115, 123, 133);
pub const ACCENT: Color32 = Color32::from_rgb(98, 182, 164);
pub const WARNING: Color32 = Color32::from_rgb(214, 166, 79);
#[allow(dead_code)] // reserved for threshold alerts
pub const CRITICAL: Color32 = Color32::from_rgb(223, 107, 103);
pub const INFO: Color32 = Color32::from_rgb(114, 167, 211);

pub fn apply(ctx: &Context, light: bool) {
    let mut style = Style::default();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.visuals = if light {
        Visuals::light()
    } else {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = WINDOW;
        visuals.window_fill = SURFACE;
        visuals.extreme_bg_color = SIDEBAR;
        visuals.faint_bg_color = SURFACE;
        visuals.override_text_color = Some(TEXT);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        visuals.widgets.inactive.bg_fill = SURFACE;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, SECONDARY);
        visuals.widgets.hovered.bg_fill = SURFACE_HOVER;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.active.bg_fill = ACCENT;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, WINDOW);
        visuals.selection.bg_fill = ACCENT.gamma_multiply(0.25);
        visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
        visuals
    };
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(5);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(5);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(5);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(5);
    ctx.set_style(style);
}
