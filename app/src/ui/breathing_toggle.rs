use egui::{Color32, Id, Response, RichText, Sense, Stroke, StrokeKind, Ui};

use super::transition::Transition;

#[derive(Clone)]
struct ToggleAnimation {
    colors: [Transition<3>; 4],
    position: Transition<1>,
    last_pass: u64,
}

fn animate(ui: &Ui, id: Id, colors: [Color32; 4], position: f32) -> ([Color32; 4], f32) {
    let now = ui.input(|input| input.time);
    let pass = ui.ctx().cumulative_pass_nr();
    let colors = colors.map(|color| [f64::from(color.r()), f64::from(color.g()), f64::from(color.b())]);
    let position = [f64::from(position)];
    let (colors, position, running) = ui.data_mut(|data| {
        let animation = data.get_temp_mut_or_insert_with(id.with("transition"), || ToggleAnimation {
            colors: colors.map(Transition::settled), position: Transition::settled(position), last_pass: pass,
        });
        if pass > animation.last_pass + 1 {
            animation.colors = colors.map(Transition::settled);
            animation.position = Transition::settled(position);
        }
        animation.last_pass = pass;
        let colors = std::array::from_fn(|index| {
            let [red, green, blue] = animation.colors[index].sample(colors[index], now);
            Color32::from_rgb(red.round() as u8, green.round() as u8, blue.round() as u8)
        });
        let position = animation.position.sample(position, now)[0] as f32;
        let running = animation.position.running(now) || animation.colors.iter().any(|color| color.running(now));
        (colors, position, running)
    });
    if running { ui.ctx().request_repaint(); }
    (colors, position)
}

pub(super) fn show(ui: &mut Ui, value: &mut bool) -> Response {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let enabled = ui.is_enabled();
        let mut painter = ui.painter().clone();
        if !enabled && painter.is_visible() { painter.set_fade_to_color(None); }
        for (text, gray) in [("Breathing", 245), (if *value { "On" } else { "Off" }, 217)] {
            let color = Color32::from_gray(if enabled { gray } else { 112 });
            let (position, galley, response) = egui::Label::new(RichText::new(text).size(12.0).strong().color(color))
                .selectable(false).layout_in_ui(ui);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, text));
            painter.galley(position, galley, color);
        }
        let (rect, _) = ui.allocate_exact_size(egui::vec2(48.0, 24.0), Sense::hover());
        let mut response = ui.interact(rect, Id::new("lighting-breathing"),
            if enabled { Sense::click() } else { Sense::hover() });
        if enabled && response.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, *value, "Breathing"));
        let hovered = enabled && response.hovered();
        let accent = Color32::from_rgb(255, 164, 13);
        let hover = Color32::from_rgb(254, 234, 210);
        let track_fill = if !enabled { Color32::from_gray(28) } else if hovered { Color32::from_gray(49) } else { Color32::from_gray(17) };
        let track_border = if !enabled { Color32::from_gray(81) } else if hovered { hover } else { Color32::from_gray(144) };
        let handle_fill = if !enabled { Color32::from_gray(81) } else if hovered { hover } else if *value { accent } else { Color32::from_gray(38) };
        let handle_border = if !enabled || hovered || *value { handle_fill } else { Color32::from_gray(217) };
        let ([track_fill, track_border, handle_fill, handle_border], offset) = animate(ui, response.id,
            [track_fill, track_border, handle_fill, handle_border], if *value { 22.0 } else { 0.0 });
        painter.rect(rect, 12.0, track_fill, Stroke::new(1.0_f32, track_border), StrokeKind::Inside);
        let center = egui::pos2(rect.left() + 13.0 + offset, rect.center().y);
        painter.circle(center, 7.5, handle_fill, Stroke::new(1.0_f32, handle_border));
        if enabled && response.has_focus() {
            painter.rect_stroke(rect.expand(2.0), 14.0,
                Stroke::new(2.0_f32, Color32::from_rgb(86, 183, 230)), StrokeKind::Outside);
        }
        if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
    }).inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(context: &egui::Context, time: f64, value: &mut bool, enabled: bool, events: Vec<egui::Event>) -> (egui::FullOutput, Response) {
        let mut response = None;
        let output = context.run(egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 150.0))),
            events,
            ..Default::default()
        }, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                response = Some(ui.add_enabled_ui(enabled, |ui| show(ui, value)).inner);
            });
        });
        (output, response.unwrap())
    }

    fn appearance(frame: &(egui::FullOutput, Response)) -> (f32, [Color32; 4]) {
        let (output, response) = frame;
        let track = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(track) if track.rect == response.rect => Some(track),
            _ => None,
        }).unwrap();
        let handle = output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Circle(handle) if response.rect.contains(handle.center) && handle.radius == 7.5 => Some(handle),
            _ => None,
        }).unwrap();
        (handle.center.x - response.rect.left() - 13.0,
            [track.fill, track.stroke.color, handle.fill, handle.stroke.color])
    }

    fn assert_offset(frame: &(egui::FullOutput, Response), expected: f32) {
        let actual = appearance(frame).0;
        assert!((actual - expected).abs() < 0.001, "handle offset {actual}, expected {expected}");
        assert!(!frame.1.changed(), "animation must not emit a brush edit");
    }

    #[test]
    fn selection_uses_300ms_css_ease_and_srgb_colors() {
        let context = egui::Context::default();
        let mut value = false;
        draw(&context, 0.0, &mut value, true, vec![]);
        value = true;
        assert_offset(&draw(&context, 1.0, &mut value, true, vec![]), 0.0);
        for (time, offset, fill) in [
            (1.046875, 4.365625, Color32::from_rgb(81, 63, 33)),
            (1.09375, 11.825, Color32::from_rgb(155, 106, 25)),
            (1.16875, 18.871875, Color32::from_rgb(224, 146, 17)),
            (1.3, 22.0, Color32::from_rgb(255, 164, 13)),
        ] {
            let frame = draw(&context, time, &mut value, true, vec![]);
            assert_offset(&frame, offset);
            assert_eq!(appearance(&frame).1[2], fill);
        }
        assert!(value);
    }

    #[test]
    fn interrupted_reversals_keep_position_and_shorten_each_return() {
        let context = egui::Context::default();
        let mut value = false;
        draw(&context, 0.0, &mut value, true, vec![]);
        value = true;
        draw(&context, 1.0, &mut value, true, vec![]);
        let forward = draw(&context, 1.09375, &mut value, true, vec![]);
        assert_offset(&forward, 11.825);
        value = false;
        let reverse = draw(&context, 1.09375, &mut value, true, vec![]);
        assert_eq!(appearance(&forward), appearance(&reverse));
        let returning = draw(&context, 1.144140625, &mut value, true, vec![]);
        assert_offset(&returning, 5.4690625);
        value = true;
        let reversed_again = draw(&context, 1.144140625, &mut value, true, vec![]);
        assert_eq!(appearance(&returning), appearance(&reversed_again));
        assert!(appearance(&draw(&context, 1.36, &mut value, true, vec![])).0 < 22.0);
        assert_offset(&draw(&context, 1.37, &mut value, true, vec![]), 22.0);
        value = false;
        assert_offset(&draw(&context, 2.0, &mut value, true, vec![]), 22.0);
        assert_offset(&draw(&context, 2.09375, &mut value, true, vec![]), 10.175);
        assert_offset(&draw(&context, 2.3, &mut value, true, vec![]), 0.0);
    }

    #[test]
    fn hover_colors_animate_independently_of_handle_motion() {
        let context = egui::Context::default();
        let mut value = false;
        let initial = draw(&context, 0.0, &mut value, true, vec![]);
        let pointer = egui::Event::PointerMoved(initial.1.rect.center());
        let hovered = draw(&context, 1.0, &mut value, true, vec![pointer]);
        assert!(hovered.1.hovered());
        assert_eq!(appearance(&initial), appearance(&hovered));
        let progress = draw(&context, 1.09375, &mut value, true, vec![]);
        assert_eq!(appearance(&progress).1, [Color32::from_gray(34), Color32::from_rgb(203, 192, 179),
            Color32::from_rgb(154, 143, 130), Color32::from_rgb(237, 226, 213)]);
        value = true;
        let selected = draw(&context, 1.09375, &mut value, true, vec![]);
        assert_eq!(appearance(&progress), appearance(&selected));
        let settled_hover = draw(&context, 1.3, &mut value, true, vec![]);
        assert_eq!(appearance(&settled_hover).1, [Color32::from_gray(49), Color32::from_rgb(254, 234, 210),
            Color32::from_rgb(254, 234, 210), Color32::from_rgb(254, 234, 210)]);
        assert!(appearance(&settled_hover).0 < 22.0);
        draw(&context, 1.3, &mut value, true, vec![egui::Event::PointerGone]);
        let leave = draw(&context, 1.3, &mut value, true, vec![]);
        assert!(!leave.1.hovered(), "pointer exit must reach the switch before advancing transition time");
        assert_eq!(appearance(&settled_hover), appearance(&leave));
        assert_offset(&draw(&context, 1.39375, &mut value, true, vec![]), 22.0);
        let settled = draw(&context, 1.6, &mut value, true, vec![]);
        assert_eq!(appearance(&settled).1[2], Color32::from_rgb(255, 164, 13));
    }

    #[test]
    fn disabling_retargets_colors_without_restarting_motion_or_accepting_input() {
        let context = egui::Context::default();
        let mut value = false;
        draw(&context, 0.0, &mut value, true, vec![]);
        value = true;
        draw(&context, 1.0, &mut value, true, vec![]);
        let moving = draw(&context, 1.09375, &mut value, true, vec![]);
        let disabled = draw(&context, 1.09375, &mut value, false, vec![]);
        assert_eq!(appearance(&moving), appearance(&disabled));
        assert!(!disabled.1.enabled());
        let midpoint = draw(&context, 1.1875, &mut value, false, vec![]);
        assert_eq!(appearance(&midpoint).1, [Color32::from_gray(23), Color32::from_gray(110),
            Color32::from_rgb(115, 92, 55), Color32::from_rgb(153, 131, 93)]);
        assert_offset(&draw(&context, 1.3, &mut value, false, vec![]), 22.0);
        let settled = draw(&context, 1.4, &mut value, false, vec![]);
        assert_eq!(appearance(&settled).1, [Color32::from_gray(28), Color32::from_gray(81),
            Color32::from_gray(81), Color32::from_gray(81)]);
        for pressed in [true, false] {
            let frame = draw(&context, 1.5, &mut value, false, vec![
                egui::Event::PointerMoved(settled.1.rect.center()),
                egui::Event::PointerButton { pos: settled.1.rect.center(), button: egui::PointerButton::Primary,
                    pressed, modifiers: Default::default() },
            ]);
            assert!(!frame.1.changed());
        }
        assert!(value);
    }

    #[test]
    fn initial_and_remounted_controls_show_the_current_state_without_animation() {
        let context = egui::Context::default();
        let mut value = true;
        assert_offset(&draw(&context, 0.0, &mut value, true, vec![]), 22.0);
        value = false;
        draw(&context, 1.0, &mut value, true, vec![]);
        value = true;
        assert_offset(&draw(&context, 1.0, &mut value, true, vec![]), 22.0);
        value = false;
        draw(&context, 2.0, &mut value, true, vec![]);
        let _ = context.run(egui::RawInput { time: Some(2.05), ..Default::default() }, |_| {});
        assert_offset(&draw(&context, 2.1, &mut value, true, vec![]), 0.0);
        value = true;
        draw(&context, 3.0, &mut value, true, vec![]);
        assert_offset(&draw(&context, 100.0, &mut value, true, vec![]), 22.0);
    }

    #[test]
    fn animation_requests_frames_only_while_a_property_is_moving() {
        let context = egui::Context::default();
        let mut value = false;
        draw(&context, 0.0, &mut value, true, vec![]);
        value = true;
        draw(&context, 1.0, &mut value, true, vec![]);
        let moving = draw(&context, 1.1, &mut value, true, vec![]);
        assert_eq!(moving.0.viewport_output[&egui::ViewportId::ROOT].repaint_delay, std::time::Duration::ZERO);
        for _ in 0..3 { draw(&context, 2.0, &mut value, true, vec![]); }
        let settled = draw(&context, 2.0, &mut value, true, vec![]);
        assert!(settled.0.viewport_output[&egui::ViewportId::ROOT].repaint_delay > std::time::Duration::ZERO);
    }

    #[test]
    fn reference_disabled_colors_do_not_make_an_invisible_switch_visible() {
        let context = egui::Context::default();
        let mut value = true;
        let output = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| {
                ui.set_invisible();
                let response = show(ui, &mut value);
                assert!(!response.enabled());
                assert!(!response.changed());
            });
        });
        assert!(output.shapes.iter().all(|shape| !matches!(&shape.shape, egui::Shape::Text(_) | egui::Shape::Circle(_))));
        assert!(value);
    }
}
