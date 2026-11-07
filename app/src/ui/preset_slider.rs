use eframe::egui;

pub(super) fn show(ui: &mut egui::Ui, scope: impl std::hash::Hash, value: &mut u8, minimum: u8) -> bool {
    let before = *value;
    ui.push_id(scope, |ui| {
        let text_id = ui.make_persistent_id("value");
        let editing = ui.ctx().read_response(text_id).is_some_and(|response| response.has_focus() || response.lost_focus());
        let pending = ui.data_mut(|data| data.get_temp::<(u8, String)>(text_id));
        let mut text = pending.filter(|(observed, _)| editing && *observed == before)
            .map(|(_, text)| text).unwrap_or_else(|| before.to_string());
        ui.horizontal(|ui| {
            let mut position = if editing { parse_number(&text).unwrap_or(minimum as f64) } else { *value as f64 };
            let slider = ui.add(egui::Slider::new(&mut position, minimum as f64..=100.0)
                .show_value(false).step_by(1.0).smart_aim(false).clamping(egui::SliderClamping::Edits));
            if slider.changed() {
                *value = stepped(position, minimum);
                text = value.to_string();
            }
            let input = ui.add(egui::TextEdit::singleline(&mut text).id(text_id).desired_width(48.0));
            if input.changed() {
                text = parse_number(&text).map(|number| {
                    if number == f64::INFINITY { "Infinity".to_string() }
                    else if number == f64::NEG_INFINITY { "-Infinity".to_string() }
                    else { number.to_string() }
                }).unwrap_or_default();
            }
            if input.lost_focus() {
                *value = stepped(parse_number(&text).unwrap_or(minimum as f64), minimum);
                text = value.to_string();
            }
            if input.has_focus() {
                ui.data_mut(|data| data.insert_temp(text_id, (*value, text)));
            } else {
                ui.data_mut(|data| data.remove::<(u8, String)>(text_id));
            }
        });
    });
    before != *value
}

fn stepped(value: f64, minimum: u8) -> u8 {
    ((value / 5.0).round() * 5.0).clamp(minimum as f64, 100.0) as u8
}

fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    let bytes = text.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    if text.get(end..)?.starts_with("Infinity") {
        return Some(if bytes.first() == Some(&b'-') { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    let integer_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
    let mut digits = end - integer_start;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let fraction_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
        digits += end - fraction_start;
    }
    if digits == 0 { return None; }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let exponent_start = end;
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) { end += 1; }
        let digits_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
        if end == digits_start { end = exponent_start; }
    }
    text[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_edits_use_reference_steps_and_bounds() {
        for (minimum, cases) in [
            (0, vec![(0.0, 0), (6.0, 5), (43.0, 45), (67.0, 65), (100.0, 100)]),
            (1, vec![(1.0, 1), (6.0, 5), (43.0, 45), (67.0, 65), (100.0, 100)]),
        ] {
            let context = egui::Context::default();
            let mut value = 30;
            let mut render = |events| {
                context.run(egui::RawInput { events, ..Default::default() }, |context| {
                    egui::CentralPanel::default().show(context, |ui| { show(ui, "test", &mut value, minimum); });
                })
            };
            for (requested, expected) in cases {
                let output = render(vec![]);
                let rail = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rectangle) if rectangle.rect.width() > 50.0 && rectangle.rect.height() < 10.0 => Some(rectangle.rect),
                    _ => None,
                }).unwrap();
                let radius = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Circle(circle) => Some(circle.radius),
                    _ => None,
                }).unwrap();
                let fraction = (requested - minimum as f32) / (100 - minimum) as f32;
                let position = egui::pos2(rail.left() + radius + fraction * (rail.width() - 2.0 * radius), rail.center().y);
                render(vec![egui::Event::PointerMoved(position), egui::Event::PointerButton {
                    pos: position, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default(),
                }]);
                let output = render(vec![egui::Event::PointerButton {
                    pos: position, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default(),
                }]);
                assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::Shape::Text(text) if text.galley.text() == expected.to_string())),
                    "pointer input {requested} with minimum {minimum} must display {expected}");
            }
        }
    }
}
