//! Shared sizing and styling for the Shader Policy Editor.

#[derive(Clone, Copy)]
pub struct EditorMetrics {
    pub scale: f32,
    pub panel_gap: f32,
    pub panel_margin: i8,
    pub row_gap: f32,
    pub label_width: f32,
    pub slider_value_width: f32,
    pub dropdown_width: f32,
    pub action_button_width: f32,
    pub status_height: f32,
}

impl EditorMetrics {
    pub fn new(scale: f32) -> Self {
        Self {
            scale,
            panel_gap: 8.0 * scale,
            panel_margin: (10.0 * scale).round() as i8,
            row_gap: 5.0 * scale,
            label_width: 116.0 * scale,
            slider_value_width: 62.0 * scale,
            dropdown_width: 138.0 * scale,
            action_button_width: 128.0 * scale,
            status_height: 28.0 * scale,
        }
    }
}

pub fn panel_frame(
    ui: &egui::Ui,
    metrics: EditorMetrics,
) -> egui::Frame {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::same(metrics.panel_margin))
}

pub fn section_heading(
    ui: &mut egui::Ui,
    text: &str,
) {
    ui.label(
        egui::RichText::new(text)
            .strong()
            .color(egui::Color32::from_rgb(96, 165, 250)),
    );
}

pub fn configure_editor_style(
    context: &egui::Context,
    resolution_scale: f32,
) {
    let mut style = (*context.style()).clone();

    style.spacing.item_spacing = egui::vec2(
        7.0 * resolution_scale,
        4.0 * resolution_scale,
    );

    style.spacing.window_margin = egui::Margin::same(
        (8.0 * resolution_scale).round() as i8,
    );

    style.spacing.button_padding = egui::vec2(
        8.0 * resolution_scale,
        3.0 * resolution_scale,
    );

    style.spacing.interact_size.y = 22.0 * resolution_scale;
    style.visuals.resize_corner_size = 12.0 * resolution_scale;

    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(14.0 * resolution_scale),
    );

    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(14.0 * resolution_scale),
    );

    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(16.0 * resolution_scale),
    );

    context.set_style(style);
}



/// Draw the pending-Bulk-Edit cue without changing the control's normal fill.
/// Orange has one meaning in Control Center: this field will be applied to
/// every checked policy if Bulk Edit is saved.
pub fn paint_bulk_edit_border(
    ui: &egui::Ui,
    rect: egui::Rect,
    scale: f32,
) {
    ui.painter().rect_stroke(
        rect.expand(1.0 * scale),
        2.0,
        egui::Stroke::new(
            (2.0 * scale).max(1.0),
            egui::Color32::from_rgb(255, 165, 0),
        ),
        egui::StrokeKind::Outside,
    );
}
