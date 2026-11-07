use super::transition::Transition;
use egui::{Color32, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, WidgetText};

const LABELS: [&str; 3] = ["Layer 1", "Layer 2", "Layer 3"];
const HELP_TEXT: &str = "Your keyboard has 3 total layers including two function (fn) layers. Each key can be remapped on each layer to a different function to open up over 240 possible key functions. Function layers are accessible by holding the fn key (and fn2 key for the fn2 layer) and pressing the intended key. If nothing is bound on an fn layer, the key function will revert to the layer below.";

#[derive(Clone)]
struct HelpVisibility {
    amount: Transition<1, 120>,
    last_pass: u64,
}

fn help(ui: &Ui, rect: Rect, id: Id) {
    let response = ui.interact(rect, id, Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Information tooltip: Keyboard Layer"));
    let color = Color32::from_gray(112);
    let center = rect.center();
    ui.painter().circle_stroke(center, 6.75, Stroke::new(1.5_f32, color));
    ui.painter().circle_filled(center - egui::vec2(0.0, 3.0), 0.75, color);
    ui.painter().line_segment([center - egui::vec2(0.0, 0.75), center + egui::vec2(0.0, 3.75)], Stroke::new(1.5_f32, color));
    if response.has_focus() {
        ui.painter().rect_stroke(rect.expand(3.0), 0.0, Stroke::new(2.0_f32, Color32::from_rgb(86, 183, 230)), StrokeKind::Middle);
    }
    let now = ui.input(|input| input.time);
    let pass = ui.ctx().cumulative_pass_nr();
    let hovered = response.hovered() && ui.input(|input| input.pointer.hover_pos().is_some());
    let target = [if ui.is_enabled() && (hovered || response.has_focus()) { 1.0 } else { 0.0 }];
    let (amount, running) = ui.data_mut(|data| {
        let visibility = data.get_temp_mut_or_insert_with(id.with("visibility"), || HelpVisibility {
            amount: Transition::settled([0.0]), last_pass: pass,
        });
        if pass > visibility.last_pass + 1 { visibility.amount = Transition::settled([0.0]); }
        visibility.last_pass = pass;
        (visibility.amount.sample(target, now)[0] as f32, visibility.amount.running(now))
    });
    if running { ui.ctx().request_repaint(); }
    if amount <= 0.0 { return; }
    let mut painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, id));
    painter.set_opacity(amount);
    let layout_text = |text: &str, size: f32, height: f32| {
        let mut job = egui::text::LayoutJob::simple(text.into(), egui::FontId::proportional(size), Color32::from_gray(217), 261.0);
        job.sections[0].format.line_height = Some(height);
        painter.layout_job(job)
    };
    let title = layout_text("Keyboard Layer", 14.0, 20.0);
    let description = layout_text(HELP_TEXT, 12.0, 16.0);
    let popup = Rect::from_min_size(egui::pos2(rect.right() - 275.0, rect.bottom() - 5.0 * (1.0 - amount)),
        egui::vec2(275.0, 14.0 + title.size().y + description.size().y));
    painter.rect(popup, 2.0, Color32::from_gray(38), Stroke::new(1.0_f32, Color32::from_rgb(132, 119, 190)), StrokeKind::Inside);
    let content_start = popup.min + egui::vec2(7.0, 7.0);
    let title_height = title.size().y;
    painter.galley(content_start, title, Color32::from_gray(217));
    painter.galley(content_start + egui::vec2(0.0, title_height), description, Color32::from_gray(217));
}

#[derive(Clone)]
struct Colors {
    background_alpha: Transition<1, 120>,
    foreground: Transition<3, 120>,
    last_pass: u64,
}

fn animate(ui: &Ui, id: Id, selected: bool, hovered: bool) -> (Color32, Color32) {
    let alpha = [if selected { 255.0 } else { 0.0 }];
    let foreground = if hovered { [235.0; 3] } else if selected { [26.0, 16.0, 3.0] } else { [168.0; 3] };
    let now = ui.input(|input| input.time);
    let pass = ui.ctx().cumulative_pass_nr();
    let (alpha, foreground, running) = ui.data_mut(|data| {
        let colors = data.get_temp_mut_or_insert_with(id.with("colors"), || Colors {
            background_alpha: Transition::settled(alpha), foreground: Transition::settled(foreground), last_pass: pass,
        });
        if pass > colors.last_pass + 1 {
            colors.background_alpha = Transition::settled(alpha);
            colors.foreground = Transition::settled(foreground);
        }
        colors.last_pass = pass;
        (colors.background_alpha.sample(alpha, now)[0], colors.foreground.sample(foreground, now),
            colors.background_alpha.running(now) || colors.foreground.running(now))
    });
    if running { ui.ctx().request_repaint(); }
    (Color32::from_rgba_unmultiplied(245, 164, 58, alpha.round() as u8),
        Color32::from_rgb(foreground[0].round() as u8, foreground[1].round() as u8, foreground[2].round() as u8))
}

pub(super) fn show(ui: &mut Ui, layer: &mut u8) -> bool {
    let before = *layer;
    let id = Id::new("keyboard-layers");
    let font = egui::FontId::proportional(13.0);
    let button_width = LABELS.iter().map(|label| {
        ui.painter().layout_no_wrap((*label).into(), font.clone(), Color32::WHITE).size().x + 36.0
    }).fold(72.0, f32::max);
    let heading = WidgetText::from(RichText::new("Keyboard Layer").size(13.0).strong().color(Color32::from_gray(235)))
        .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font.clone());
    let group_width = button_width * 3.0 + 12.0;
    let heading_height = heading.size().y.max(18.0);
    let (area, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(group_width), heading_height + 8.0 + 38.0), Sense::hover());
    let help_rect = Rect::from_min_size(egui::pos2(area.right() - 18.0, area.top() + (heading_height - 18.0) * 0.5), egui::vec2(18.0, 18.0));
    ui.painter().galley(egui::pos2(help_rect.left() - 6.0 - heading.size().x, area.top() + (heading_height - heading.size().y) * 0.5), heading, Color32::from_gray(235));
    help(ui, help_rect, id.with("help"));
    let group = Rect::from_min_size(egui::pos2(area.right() - group_width, area.bottom() - 38.0), egui::vec2(group_width, 38.0));
    let responses: Vec<_> = (0usize..3).map(|index| {
        let rect = Rect::from_min_size(group.min + egui::vec2(4.0 + index as f32 * (button_width + 2.0), 4.0), egui::vec2(button_width, 30.0));
        let response = ui.interact(rect, id.with(index), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() { *layer = index as u8; }
        response
    }).collect();
    ui.painter().rect(group, 19.0, Color32::from_gray(19), Stroke::new(1.0_f32, Color32::from_gray(38)), StrokeKind::Inside);
    for (index, response) in responses.into_iter().enumerate() {
        let selected = *layer == index as u8;
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, ui.is_enabled(), selected, LABELS[index]));
        let (background, foreground) = animate(ui, response.id, selected, response.hovered());
        if selected {
            ui.painter().add(egui::epaint::Shadow { offset: [0, 1], blur: 2, spread: 0,
                color: Color32::from_black_alpha(89) }.as_shape(response.rect, 15.0));
        }
        ui.painter().rect_filled(response.rect, 15.0, background);
        let galley = WidgetText::from(RichText::new(LABELS[index]).size(13.0).strong().color(foreground))
            .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, font.clone());
        ui.painter().galley(response.rect.center() - galley.size() * 0.5, galley, foreground);
        if response.has_focus() {
            ui.painter().rect_stroke(response.rect, 15.0, Stroke::new(1.0_f32, Color32::from_rgb(86, 183, 230)), StrokeKind::Inside);
        }
    }
    *layer != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(context: &egui::Context, time: f64, layer: &mut u8, enabled: bool, events: Vec<egui::Event>) -> (egui::FullOutput, bool) {
        let mut changed = false;
        let output = context.run(egui::RawInput {
            time: Some(time), screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 150.0))),
            events, ..Default::default()
        }, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                ui.set_max_width(400.0);
                ui.add_enabled_ui(enabled, |ui| { changed = show(ui, layer); });
            });
        });
        (output, changed)
    }

    fn button(context: &egui::Context, index: usize) -> egui::Response {
        context.read_response(Id::new("keyboard-layers").with(index)).unwrap()
    }

    fn text<'a>(output: &'a egui::FullOutput, label: &str) -> &'a egui::epaint::TextShape {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => Some(text), _ => None,
        }).unwrap()
    }

    fn fill(output: &egui::FullOutput, rect: Rect) -> Color32 {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(shape) if shape.rect == rect && shape.stroke.width == 0.0 => Some(shape.fill), _ => None,
        }).unwrap()
    }

    #[test]
    fn group_has_padded_equal_buttons_and_right_aligned_heading() {
        let context = egui::Context::default();
        let (output, changed) = draw(&context, 0.0, &mut 0, true, vec![]);
        assert!(!changed);
        let first = button(&context, 0).rect;
        let second = button(&context, 1).rect;
        let third = button(&context, 2).rect;
        assert!(first.width() >= 72.0);
        assert_eq!(first.height(), 30.0);
        assert_eq!(first.size(), second.size());
        assert_eq!(second.size(), third.size());
        assert_eq!(second.left() - first.right(), 2.0);
        assert_eq!(third.left() - second.right(), 2.0);
        let group = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == Color32::from_gray(19) => Some(rect.rect), _ => None,
        }).unwrap();
        assert_eq!(first.min, group.min + egui::vec2(4.0, 4.0));
        assert_eq!(third.right(), group.right() - 4.0);
        let heading = text(&output, "Keyboard Layer");
        let help = context.read_response(Id::new("keyboard-layers").with("help")).unwrap();
        assert_eq!(help.rect.size(), egui::vec2(18.0, 18.0));
        assert_eq!(help.rect.right(), group.right());
        assert_eq!(heading.pos.x + heading.galley.size().x + 6.0, help.rect.left());
        assert_eq!(group.top() - help.rect.bottom(), 8.0);
        for label in LABELS { assert_eq!(text(&output, label).galley.job.sections[0].format.font_id.size, 13.0); }
    }

    #[test]
    fn layer_help_opens_below_the_icon_on_hover_and_keyboard_focus_without_selecting_a_layer() {
        let context = egui::Context::default();
        let mut layer = 1;
        let (initial, _) = draw(&context, 0.0, &mut layer, true, vec![]);
        let help_id = Id::new("keyboard-layers").with("help");
        let help = context.read_response(help_id).unwrap().rect;
        let description = "Your keyboard has 3 total layers including two function (fn) layers. Each key can be remapped on each layer to a different function to open up over 240 possible key functions. Function layers are accessible by holding the fn key (and fn2 key for the fn2 layer) and pressing the intended key. If nothing is bound on an fn layer, the key function will revert to the layer below.";
        assert!(!initial.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == description)));
        draw(&context, 1.0, &mut layer, true, vec![egui::Event::PointerMoved(help.center())]);
        let (hover, changed) = draw(&context, 1.12, &mut layer, true, vec![]);
        assert!(!changed);
        assert_eq!(layer, 1);
        let popup = hover.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.stroke.color == Color32::from_rgb(132, 119, 190) => Some(rect), _ => None,
        }).unwrap();
        assert_eq!(popup.rect.width(), 275.0);
        assert_eq!(popup.rect.right(), help.right());
        assert_eq!(popup.rect.top(), help.bottom());
        assert_eq!(popup.fill, Color32::from_gray(38));
        let content = text(&hover, description);
        assert_eq!(content.galley.job.sections[0].format.font_id.size, 12.0);
        assert_eq!(content.galley.job.sections[0].format.line_height, Some(16.0));
        assert_eq!(content.pos.x, popup.rect.left() + 7.0);
        assert_eq!(content.pos.y, popup.rect.top() + 27.0);
        context.memory_mut(|memory| memory.request_focus(help_id));
        let (focused, changed) = draw(&context, 2.0, &mut layer, true, vec![egui::Event::PointerGone]);
        assert!(!changed);
        assert_eq!(text(&focused, description).galley.job.text, description);
        context.memory_mut(|memory| memory.surrender_focus(help_id));
        draw(&context, 3.0, &mut layer, true, vec![]);
        let (closed, _) = draw(&context, 3.12, &mut layer, true, vec![]);
        assert!(!closed.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == description)));
        assert_eq!(layer, 1);
    }

    #[test]
    fn help_fade_moves_five_points_continuously_and_does_not_reappear_after_remount() {
        let context = egui::Context::default();
        let mut layer = 2;
        draw(&context, 0.0, &mut layer, true, vec![]);
        let help = context.read_response(Id::new("keyboard-layers").with("help")).unwrap().rect;
        draw(&context, 1.0, &mut layer, true, vec![egui::Event::PointerMoved(help.center())]);
        let popup_rect = |output: &egui::FullOutput| output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.rect.width() == 275.0 => Some(rect.rect), _ => None,
        });
        let (opening, _) = draw(&context, 1.0375, &mut layer, true, vec![]);
        let position = popup_rect(&opening).unwrap().top();
        assert!(position > help.bottom() - 5.0 && position < help.bottom());
        let (reversed, _) = draw(&context, 1.0375, &mut layer, true, vec![egui::Event::PointerGone]);
        assert_eq!(popup_rect(&reversed).unwrap().top(), position);
        let (closed, _) = draw(&context, 1.103, &mut layer, true, vec![]);
        assert!(popup_rect(&closed).is_none());
        draw(&context, 2.0, &mut layer, true, vec![egui::Event::PointerMoved(help.center())]);
        draw(&context, 2.12, &mut layer, true, vec![]);
        let _ = context.run(egui::RawInput { time: Some(3.0), events: vec![egui::Event::PointerGone], ..Default::default() }, |_| {});
        let (remounted, _) = draw(&context, 3.01, &mut layer, true, vec![]);
        assert!(popup_rect(&remounted).is_none());
        let (disabled, changed) = draw(&context, 4.0, &mut layer, false, vec![egui::Event::PointerMoved(help.center())]);
        assert!(popup_rect(&disabled).is_none());
        assert!(!changed);
        assert_eq!(layer, 2);
    }

    #[test]
    fn selection_colors_follow_120ms_ease_and_interrupted_reversal() {
        let context = egui::Context::default();
        let mut layer = 0;
        draw(&context, 0.0, &mut layer, true, vec![]);
        layer = 1;
        draw(&context, 1.0, &mut layer, true, vec![]);
        let (midpoint, _) = draw(&context, 1.0375, &mut layer, true, vec![]);
        let rect = button(&context, 1).rect;
        assert_eq!(fill(&midpoint, rect), Color32::from_rgba_unmultiplied(245, 164, 58, 137));
        assert_eq!(text(&midpoint, "Layer 2").galley.job.sections[0].format.color, Color32::from_rgb(92, 86, 79));
        layer = 0;
        let (reversed, _) = draw(&context, 1.0375, &mut layer, true, vec![]);
        assert_eq!(fill(&reversed, rect), fill(&midpoint, rect));
        let (finished, _) = draw(&context, 1.103, &mut layer, true, vec![]);
        assert_eq!(fill(&finished, rect).a(), 0);
        assert_eq!(text(&finished, "Layer 2").galley.job.sections[0].format.color, Color32::from_gray(168));
    }

    #[test]
    fn hover_overrides_active_text_color_without_changing_selection() {
        let context = egui::Context::default();
        let mut layer = 0;
        draw(&context, 0.0, &mut layer, true, vec![]);
        let position = button(&context, 0).rect.center();
        draw(&context, 1.0, &mut layer, true, vec![egui::Event::PointerMoved(position)]);
        let (hover, changed) = draw(&context, 1.12, &mut layer, true, vec![]);
        assert_eq!(layer, 0);
        assert!(!changed);
        assert_eq!(text(&hover, "Layer 1").galley.job.sections[0].format.color, Color32::from_gray(235));
        assert_eq!(fill(&hover, button(&context, 0).rect), Color32::from_rgb(245, 164, 58));
    }

    #[test]
    fn remount_uses_current_layer_colors_without_resuming_a_previous_transition() {
        let context = egui::Context::default();
        let mut layer = 0;
        draw(&context, 0.0, &mut layer, true, vec![]);
        layer = 1;
        draw(&context, 1.0, &mut layer, true, vec![]);
        let _ = context.run(egui::RawInput { time: Some(1.01), ..Default::default() }, |_| {});
        let (output, _) = draw(&context, 1.02, &mut layer, true, vec![]);
        assert_eq!(fill(&output, button(&context, 1).rect), Color32::from_rgb(245, 164, 58));
        assert_eq!(text(&output, "Layer 2").galley.job.sections[0].format.color, Color32::from_rgb(26, 16, 3));
    }

    #[test]
    fn keyboard_selects_each_layer_and_repeated_selection_is_unchanged() {
        let context = egui::Context::default();
        let mut layer = 0;
        draw(&context, 0.0, &mut layer, true, vec![]);
        for (step, index) in [2usize, 1, 0, 0].into_iter().enumerate() {
            let before = layer;
            let time = 1.0 + step as f64;
            context.memory_mut(|memory| memory.request_focus(Id::new("keyboard-layers").with(index)));
            let key = if index == 1 { egui::Key::Space } else { egui::Key::Enter };
            let (_, changed) = draw(&context, time, &mut layer, true, vec![egui::Event::Key {
                key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(layer, index as u8);
            assert_eq!(changed, before != layer);
            draw(&context, time, &mut layer, true, vec![egui::Event::Key {
                key, physical_key: None, pressed: false, repeat: false, modifiers: egui::Modifiers::NONE,
            }]);
        }
    }

    #[test]
    fn disabled_pointer_click_does_not_change_the_layer() {
        let context = egui::Context::default();
        let mut layer = 0;
        draw(&context, 0.0, &mut layer, false, vec![]);
        let response = button(&context, 2);
        assert!(!response.enabled());
        for pressed in [true, false] {
            let (_, changed) = draw(&context, 1.0, &mut layer, false, vec![egui::Event::PointerMoved(response.rect.center()), egui::Event::PointerButton {
                pos: response.rect.center(), button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
            }]);
            assert!(!changed);
            assert_eq!(layer, 0);
        }
    }
}
