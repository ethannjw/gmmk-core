use crate::protocol::{LED_EFFECT_IDS, LED_EFFECT_LABELS};
use super::appearance::{DEVICE_TAB_ACCENT, PANEL, RAIL, TEXT};
use egui::{Color32, FontId, Rect, Sense, Stroke, Ui, Vec2};

pub(super) const WIDTH: f32 = 384.0;
const PAGE_SIZE: usize = 9;
const TILE_SIZE: Vec2 = egui::vec2(120.0, 40.0);
const TILE_GAP: Vec2 = egui::vec2(12.0, 12.0);

pub(super) fn initial_page(effect: u8) -> usize {
    LED_EFFECT_IDS.iter().position(|candidate| *candidate == effect).unwrap_or(0) / PAGE_SIZE
}

pub(super) fn show(ui: &mut Ui, effect: &mut u8, page: &mut usize) -> bool {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(egui::RichText::new("Base Effect").size(14.0).strong().color(TEXT));
        ui.add_space(28.0);
        let visible = LED_EFFECT_IDS.iter().copied().enumerate().filter(|(_, value)| *value != 18);
        let page_count = visible.clone().count().div_ceil(PAGE_SIZE);
        let grid_height = if *page < page_count { 144.0 } else { 0.0 };
        let (grid, _) = ui.allocate_exact_size(egui::vec2(WIDTH, grid_height), Sense::hover());
        let mut changed = false;
        for (position, (index, value)) in visible.skip(*page * PAGE_SIZE).take(PAGE_SIZE).enumerate() {
            let offset = egui::vec2((position % 3) as f32, (position / 3) as f32) * (TILE_SIZE + TILE_GAP);
            let rect = Rect::from_min_size(grid.min + offset, TILE_SIZE);
            let label = if index == 0 { "LED Off" } else { LED_EFFECT_LABELS[index] };
            let selected = *effect == value;
            let response = ui.interact(rect, ui.id().with(("preset-effect", value)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, label));
            if response.clicked() {
                changed |= !selected;
                *effect = value;
            }
            ui.painter().rect(rect, 3.0, if selected { Color32::from_rgb(197, 180, 227) } else { PANEL },
                Stroke::new(1.0_f32, Color32::from_gray(81)), egui::StrokeKind::Inside);
            let color = if selected { RAIL } else { TEXT };
            let text = ui.painter().layout(label.to_string(), FontId::proportional(12.0), color, rect.width() - 32.0);
            ui.painter().galley(rect.center() - text.size() / 2.0, text, color);
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        ui.add_space(20.0);
        let navigation_width = 48.0 + page_count as f32 * 8.0 + (page_count + 1) as f32 * 8.0;
        let (row, _) = ui.allocate_exact_size(egui::vec2(WIDTH, 24.0), Sense::hover());
        let previous = Rect::from_min_size(egui::pos2(row.right() - navigation_width, row.top()), egui::vec2(24.0, 24.0));
        if arrow(ui, previous, false) && *page > 0 { *page -= 1; }
        for index in 0..page_count {
            let rect = Rect::from_min_size(previous.right_top() + egui::vec2(8.0 + index as f32 * 16.0, 8.0), egui::vec2(8.0, 8.0));
            let response = ui.interact(rect, ui.id().with(("preset-effect-page", index)), Sense::click());
            response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), *page == index,
                format!("Effect page {}", index + 1)));
            if response.clicked() { *page = index; }
            ui.painter().circle_filled(rect.center(), 4.0, if *page == index { DEVICE_TAB_ACCENT } else { Color32::from_gray(144) });
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        let next = Rect::from_min_size(row.right_top() - egui::vec2(24.0, 0.0), egui::vec2(24.0, 24.0));
        if arrow(ui, next, true) && *page + 1 < page_count { *page += 1; }
        changed
    }).inner
}

fn arrow(ui: &mut Ui, rect: Rect, next: bool) -> bool {
    let name = if next { "preset-effect-next" } else { "preset-effect-previous" };
    let label = if next { "Next effects page" } else { "Previous effects page" };
    let response = ui.interact(rect, egui::Id::new(name), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    let direction = if next { -1.0 } else { 1.0 };
    let center = rect.center();
    let tip = center - egui::vec2(6.0 * direction, 0.0);
    let stroke = Stroke::new(1.5_f32, Color32::WHITE);
    for end in [center + egui::vec2(0.0, -6.0), center + egui::vec2(0.0, 6.0), center + egui::vec2(6.0 * direction, 0.0)] {
        ui.painter().line_segment([tip, end], stroke);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}
