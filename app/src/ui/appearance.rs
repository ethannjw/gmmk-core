use crate::layout::KeyboardLayout;
use crate::profile::LightingState;
use eframe::egui::{self, Color32, FontId, Rect, Stroke};

pub(super) const BACKGROUND: Color32 = Color32::from_rgb(17, 17, 17);
pub(super) const RAIL: Color32 = Color32::from_rgb(28, 28, 28);
pub(super) const PANEL: Color32 = Color32::from_rgb(38, 38, 38);
pub(super) const ACCENT: Color32 = Color32::from_rgb(255, 163, 0);
pub(super) const DEVICE_TAB_ACCENT: Color32 = Color32::from_rgb(255, 164, 13);
pub(super) const TEXT: Color32 = Color32::from_rgb(191, 191, 191);

pub(super) fn configure(context: &egui::Context) {
    let fonts_installed = context.data_mut(|data| {
        let id = egui::Id::new("gmmk-fonts-installed");
        let installed = data.get_temp::<bool>(id).unwrap_or(false);
        data.insert_temp(id, true);
        installed
    });
    if !fonts_installed {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Work Sans".into(), egui::FontData::from_static(include_bytes!("../../assets/fonts/WorkSans.ttf")).into());
        fonts.font_data.insert("Space Grotesk".into(), egui::FontData::from_static(include_bytes!("../../assets/fonts/SpaceGrotesk.ttf")).into());
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "Work Sans".into());
        let mut heading_fonts = fonts.families[&egui::FontFamily::Proportional].clone();
        heading_fonts.insert(0, "Space Grotesk".into());
        fonts.families.insert(heading_family(), heading_fonts);
        context.set_fonts(fonts);
    }
    let mut style = (*context.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.window_fill = RAIL;
    style.visuals.extreme_bg_color = BACKGROUND;
    style.visuals.override_text_color = Some(TEXT);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, Color32::BLACK);
    style.visuals.widgets.inactive.bg_fill = PANEL;
    style.visuals.widgets.inactive.weak_bg_fill = PANEL;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::BLACK);
    style.spacing.item_spacing = egui::vec2(10.0, 12.0);
    style.spacing.button_padding = egui::vec2(14.0, 10.0);
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(22.0));
    context.set_style(style);
}

pub(super) fn heading_family() -> egui::FontFamily {
    egui::FontFamily::Name("Space Grotesk".into())
}

pub(super) fn heading(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(17.0).strong().color(Color32::WHITE));
}

pub(super) fn choice(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let text_color = if selected { Color32::BLACK } else { TEXT };
    ui.add(egui::Button::new(egui::RichText::new(text).color(text_color))
        .fill(if selected { ACCENT } else { PANEL }).corner_radius(16.0))
}

pub(super) fn binding_tile(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let previous_style = ui.style().clone();
    let fill = if selected { Color32::from_rgb(197, 180, 227) } else { PANEL };
    let border = if selected { Color32::from_rgb(217, 206, 244) } else { Color32::from_gray(81) };
    let hover_border = if selected { Color32::from_rgb(132, 119, 190) } else { Color32::from_gray(112) };
    let text_color = if selected { RAIL } else { Color32::WHITE };
    let widgets = &mut ui.visuals_mut().widgets;
    widgets.inactive.bg_stroke = Stroke::new(1.0_f32, border);
    widgets.hovered.bg_stroke = Stroke::new(1.0_f32, hover_border);
    widgets.active.bg_stroke = Stroke::new(1.0_f32, hover_border);
    widgets.open.bg_stroke = Stroke::new(1.0_f32, border);
    widgets.inactive.expansion = 0.0;
    widgets.hovered.expansion = 0.0;
    widgets.active.expansion = 0.0;
    widgets.open.expansion = 0.0;
    ui.spacing_mut().button_padding = egui::vec2(5.0, 12.0);
    let label = egui::RichText::new(text).size(14.0).color(text_color);
    let label = if selected { label.strong() } else { label };
    let response = ui.add_sized(egui::vec2(117.0, 48.0), egui::Button::new(label).wrap().fill(fill).corner_radius(2.0));
    ui.set_style(previous_style);
    response
}

pub(super) fn keyboard(
    ui: &mut egui::Ui,
    area: Rect,
    layout: &KeyboardLayout,
    selected: Option<usize>,
    lighting: Option<&LightingState>,
    interactive: bool,
) -> Option<usize> {
    let width = layout.keys.iter().map(|key| key.x + key.w).fold(0.0, f32::max);
    let height = layout.keys.iter().map(|key| key.y + key.h).fold(0.0, f32::max);
    let scale = ((area.width() - 48.0) / width)
        .min((area.height() - 48.0) / height).clamp(0.1, 0.96);
    let size = egui::vec2(width, height) * scale;
    let board = Rect::from_center_size(area.center(), size);
    let chassis = board.expand(12.0 * scale);
    ui.painter().rect_filled(chassis.translate(egui::vec2(0.0, 4.0 * scale)), 4.0, Color32::BLACK);
    ui.painter().rect_filled(chassis, 4.0, Color32::from_gray(20));
    ui.painter().rect_stroke(chassis, 4.0, Stroke::new(1.0_f32, Color32::from_gray(65)), egui::StrokeKind::Inside);
    if lighting.is_some_and(|lighting| lighting.side_light_on) {
        for position in [chassis.left() + 5.0, chassis.right() - 5.0] {
            ui.painter().line_segment([
                egui::pos2(position, chassis.top() + 16.0),
                egui::pos2(position, chassis.bottom() - 16.0),
            ], Stroke::new(2.0_f32, Color32::from_rgb(156, 85, 255)));
        }
    }
    let mut clicked = None;
    for key in &layout.keys {
        let rect = Rect::from_min_size(
            board.min + egui::vec2(key.x, key.y) * scale,
            egui::vec2(key.w, key.h) * scale,
        );
        let response = ui.interact(rect, ui.id().with(("key", key.matrix_index)),
            if interactive { egui::Sense::click() } else { egui::Sense::hover() });
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, interactive, &key.label));
        let active = selected == Some(key.matrix_index);
        let border = if active || (interactive && response.hovered()) || response.has_focus() { ACCENT } else { Color32::from_gray(56) };
        ui.painter().rect_filled(rect, 2.0, Color32::from_gray(25));
        let cap = Rect::from_min_max(rect.min + egui::vec2(2.0, 1.5) * scale,
            rect.max - egui::vec2(2.0, 4.5) * scale);
        ui.painter().rect_filled(cap, 2.0, if active { Color32::from_rgb(82, 61, 25) } else { Color32::from_gray(43) });
        ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0_f32, border), egui::StrokeKind::Inside);
        let led = layout.led_index_for_matrix_key(&key.label);
        let custom = lighting.and_then(|lighting| lighting.per_key_colors.as_ref())
            .and_then(|colors| colors.iter().find(|color| Some(color.matrix_index) == led && color.visible != 0));
        let color = custom.map(|color| Color32::from_rgb(color.r, color.g, color.b)).unwrap_or(TEXT);
        ui.painter().text(cap.center(), egui::Align2::CENTER_CENTER, key_label(&key.label),
            FontId::proportional((9.0 * scale).max(3.5)), color);
        if response.clicked() { clicked = Some(key.matrix_index); }
        response.on_hover_text(&key.label);
    }
    clicked
}

fn key_label(label: &str) -> String {
    match label {
        "period" => "`", "dot" => ".", "k29" => "\\", "Left Win" => "Win",
        "print" => "PrtSc", "delete" => "Del", "insert" => "Ins",
        "PageUp" => "PgUp", "PageDown" => "PgDn", "Backspace" => "Back",
        "CapsLock" => "Caps", "Numlock" => "Num", "Numdivide" => "/",
        "Nummulti" => "*", "Numminus" => "-", "Numplus" => "+",
        "Numenter" => "Enter", "Numdot" => ".", "RShift" => "Shift",
        "RCtrl" => "Ctrl", "RAlt" => "Alt", "Left" => "←", "Right" => "→",
        "Up" => "↑", "Down" => "↓", "Space" => "",
        other => return other.strip_prefix("Num").unwrap_or(other).trim().to_uppercase(),
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_legends_fit_their_keys_at_card_and_editor_sizes() {
        let layout = KeyboardLayout::load_embedded().unwrap();
        let context = egui::Context::default();
        configure(&context);
        for size in [egui::vec2(290.0, 195.0), egui::vec2(560.0, 260.0), egui::vec2(820.0, 400.0)] {
            let output = context.run(egui::RawInput::default(), |context| {
                egui::CentralPanel::default().show(context, |ui| {
                    keyboard(ui, Rect::from_min_size(egui::pos2(20.0, 20.0), size), &layout, None, None, false);
                });
            });
            let keys: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == Color32::from_gray(25) => Some(rect.rect),
                _ => None,
            }).collect();
            assert_eq!(keys.len(), 99);
            for shape in &output.shapes {
                if let egui::Shape::Text(text) = &shape.shape {
                    if text.galley.text().is_empty() { continue; }
                    let bounds = text.visual_bounding_rect();
                    let key = keys.iter().find(|key| key.contains(bounds.center())).expect("legend inside key");
                    assert!(key.contains_rect(bounds), "legend {:?} overflows at {size:?}: {bounds:?} outside {key:?}", text.galley.text());
                }
            }
        }
    }
}
