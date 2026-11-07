use egui::{Color32, FontId, Rect, Sense, Ui};

pub(super) const HEIGHT: f32 = 108.0;
const LABELS: [&str; 8] = ["WASD", "Arrows", "F Keys", "Modifiers", "Alphas", "Numbers", "Side Lights", "All"];

pub(super) enum Selection {
    Group(usize),
    All,
}

pub(super) fn show(ui: &mut Ui, enabled: bool) -> Option<Selection> {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(egui::RichText::new("Quick Key Selection").size(14.0).strong().color(Color32::from_gray(245)));
        ui.add_space(5.0);
        let (grid, _) = ui.allocate_exact_size(egui::vec2(493.0, 79.0), Sense::hover());
        let mut selected = None;
        for (index, label) in LABELS.iter().enumerate() {
            let rect = Rect::from_min_size(
                grid.min + egui::vec2(8.0 + (index % 4) as f32 * 125.0, (index / 4) as f32 * 47.0),
                egui::vec2(110.0, 32.0),
            );
            let active = enabled && ui.is_enabled() && index != 6;
            let response = ui.interact(rect, egui::Id::new("lighting-quick-keys").with(index),
                if active { Sense::click() } else { Sense::hover() });
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, active, *label));
            let fill = if active { Color32::from_gray(38) } else { Color32::from_gray(112).gamma_multiply(0.7) };
            let color = if active { Color32::from_gray(217) } else { Color32::from_gray(153).gamma_multiply(0.7) };
            ui.painter().rect_filled(rect, 16.0, fill);
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, label, FontId::proportional(12.0), color);
            if active && response.has_focus() {
                ui.painter().rect_stroke(rect, 16.0, ui.visuals().selection.stroke, egui::StrokeKind::Inside);
            }
            if active && response.clicked() {
                selected = Some(if index == 7 { Selection::All } else { Selection::Group(index) });
            }
            if index == 6 {
                response.on_hover_text("The Windows quick selector has no mapped side-light components. Its intended behavior is awaiting confirmation.");
            } else if active {
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                if index == 7 {
                    response.on_hover_text("Applies to all 99 keys and 20 side LEDs in the local draft.");
                }
            }
        }
        selected
    }).inner
}
