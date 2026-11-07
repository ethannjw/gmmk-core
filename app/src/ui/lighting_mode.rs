use egui::{Color32, Id, Rect, RichText, Sense, Stroke, StrokeKind, Ui, WidgetText};

const DURATION: f64 = 0.2;
const LABELS: [&str; 2] = ["Base Effect", "Custom Per Key"];

#[derive(Clone)]
struct Slide {
    start: f64,
    end: f64,
    reversing_start: f64,
    started: f64,
    shortening: f64,
    last_pass: u64,
}

impl Slide {
    fn settled(value: f64, pass: u64) -> Self {
        Self { start: value, end: value, reversing_start: value, started: 0.0, shortening: 1.0, last_pass: pass }
    }

    fn running(&self, now: f64) -> bool {
        self.start != self.end && now < self.started + DURATION * self.shortening
    }

    fn progress(&self, now: f64) -> f64 {
        if !self.running(now) { return 1.0; }
        ease_out(((now - self.started) / (DURATION * self.shortening)).clamp(0.0, 1.0))
    }

    fn value(&self, progress: f64) -> f64 {
        if progress == 1.0 { self.end } else { self.start + (self.end - self.start) * progress }
    }

    fn sample(&mut self, target: f64, now: f64, pass: u64) -> f64 {
        if pass > self.last_pass + 1 { *self = Self::settled(target, pass); }
        self.last_pass = pass;
        let progress = self.progress(now);
        let current = self.value(progress);
        if target != self.end {
            if current == target { *self = Self::settled(target, pass); }
            else {
                let reversing = self.running(now) && target == self.reversing_start;
                self.shortening = if reversing {
                    (progress * self.shortening + 1.0 - self.shortening).abs().clamp(0.0, 1.0)
                } else { 1.0 };
                self.reversing_start = if reversing { self.end } else { current };
                self.start = current;
                self.end = target;
                self.started = now;
            }
        }
        self.value(self.progress(now))
    }
}

fn ease_out(progress: f64) -> f64 {
    if progress <= 0.0 { return 0.0; }
    if progress >= 1.0 { return 1.0; }
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..30 {
        let parameter = (lower + upper) * 0.5;
        let horizontal = parameter * parameter * (1.74 - 0.74 * parameter);
        if horizontal < progress { lower = parameter; } else { upper = parameter; }
    }
    let parameter = (lower + upper) * 0.5;
    parameter * parameter * (3.0 - 2.0 * parameter)
}

pub(super) fn show(ui: &mut Ui, custom: &mut bool) -> bool {
    let id = Id::new("lighting-mode");
    let before = *custom;
    let label_size = LABELS.iter().map(|label| {
        ui.painter().layout_no_wrap((*label).into(), egui::FontId::proportional(14.0), Color32::WHITE).size()
    }).fold(egui::Vec2::ZERO, |maximum, size| maximum.max(size));
    let width = ui.available_width().max(label_size.x * 2.0 + 8.0);
    let (track, _) = ui.allocate_exact_size(egui::vec2(width, label_size.y + 24.0), Sense::hover());
    let inner = track.shrink(4.0);
    let option_size = egui::vec2(inner.width() / 2.0, inner.height());
    let responses: Vec<_> = LABELS.iter().enumerate().map(|(index, _)| {
        let rect = Rect::from_min_size(inner.min + egui::vec2(index as f32 * option_size.x, 0.0), option_size);
        let response = ui.interact(rect, id.with(index), Sense::click());
        if response.clicked() { *custom = index == 1; }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }).collect();
    let now = ui.input(|input| input.time);
    let pass = ui.ctx().cumulative_pass_nr();
    let target = if *custom { 1.0 } else { 0.0 };
    let (position, running) = ui.data_mut(|data| {
        let slide = data.get_temp_mut_or_insert_with(id, || Slide::settled(target, pass));
        let position = slide.sample(target, now, pass);
        (position as f32, slide.running(now))
    });
    if running { ui.ctx().request_repaint(); }
    ui.painter().rect(track, 3.0, Color32::from_gray(28), Stroke::new(1.0_f32, Color32::from_gray(38)), StrokeKind::Inside);
    let indicator = Rect::from_min_size(inner.min + egui::vec2(position * option_size.x, 0.0), option_size);
    ui.painter().rect_filled(indicator, 3.0, Color32::from_gray(49));
    for (index, response) in responses.into_iter().enumerate() {
        let selected = *custom == (index == 1);
        response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button,
            ui.is_enabled(), selected, LABELS[index]));
        let color = if selected { Color32::from_rgb(217, 206, 244) } else { Color32::from_gray(217) };
        let text = RichText::new(LABELS[index]).size(14.0).color(color);
        let galley = WidgetText::from(if selected { text.strong() } else { text })
            .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::FontId::proportional(14.0));
        ui.painter().galley(response.rect.center() - galley.size() * 0.5, galley, color);
        if response.has_focus() {
            ui.painter().rect_stroke(response.rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(86, 183, 230)), StrokeKind::Inside);
        }
    }
    *custom != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 0.000001, "{actual} != {expected}"); }

    fn draw(context: &egui::Context, time: f64, custom: &mut bool, enabled: bool, events: Vec<egui::Event>) -> (egui::FullOutput, bool) {
        let mut changed = false;
        let output = context.run(egui::RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 150.0))),
            events, ..Default::default()
        }, |context| {
            egui::CentralPanel::default().show(context, |ui| {
                ui.set_max_width(384.0);
                ui.add_enabled_ui(enabled, |ui| { changed = show(ui, custom); });
            });
        });
        (output, changed)
    }

    fn shape(output: &egui::FullOutput, gray: u8) -> &egui::epaint::RectShape {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect) if rect.fill == Color32::from_gray(gray) && rect.rect.height() < 60.0 => Some(rect),
            _ => None,
        }).unwrap()
    }

    #[test]
    fn rendered_indicator_tracks_ease_out_and_hover_does_not_restart_it() {
        let context = egui::Context::default();
        let mut custom = false;
        let (initial, changed) = draw(&context, 0.0, &mut custom, true, vec![]);
        assert!(!changed);
        let track = shape(&initial, 28).rect;
        let first = shape(&initial, 49).rect;
        assert_eq!(first.min, track.min + egui::vec2(4.0, 4.0));
        assert_eq!(first.height(), track.height() - 8.0);
        assert_eq!(first.width(), (track.width() - 8.0) / 2.0);
        custom = true;
        draw(&context, 1.0, &mut custom, true, vec![]);
        let (half, changed) = draw(&context, 1.0685, &mut custom, true, vec![egui::Event::PointerMoved(track.center())]);
        assert!(!changed);
        close(f64::from(shape(&half, 49).rect.left() - first.left()) / f64::from(first.width()), 0.5);
        let (finished, _) = draw(&context, 1.2, &mut custom, true, vec![]);
        assert_eq!(shape(&finished, 49).rect.right(), track.right() - 4.0);
        assert_eq!(shape(&finished, 49).corner_radius, egui::CornerRadius::same(3));
    }

    #[test]
    fn keyboard_activates_each_mode_and_repeat_selection_is_unchanged() {
        let context = egui::Context::default();
        let mut custom = false;
        draw(&context, 0.0, &mut custom, true, vec![]);
        for (step, (index, key)) in [(1usize, egui::Key::Enter), (1, egui::Key::Space), (0, egui::Key::Enter)].into_iter().enumerate() {
            let time = 1.0 + step as f64;
            let before = custom;
            context.memory_mut(|memory| memory.request_focus(Id::new("lighting-mode").with(index)));
            let (_, changed) = draw(&context, time, &mut custom, true, vec![egui::Event::Key {
                key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE,
            }]);
            assert_eq!(custom, index == 1);
            assert_eq!(changed, before != custom);
            draw(&context, time, &mut custom, true, vec![egui::Event::Key {
                key, physical_key: None, pressed: false, repeat: false, modifiers: egui::Modifiers::NONE,
            }]);
        }
    }

    #[test]
    fn disabled_pointer_activation_cannot_change_the_mode() {
        let context = egui::Context::default();
        let mut custom = false;
        draw(&context, 0.0, &mut custom, false, vec![]);
        let response = context.read_response(Id::new("lighting-mode").with(1usize)).unwrap();
        let position = response.rect.center();
        assert!(!response.enabled());
        for pressed in [true, false] {
            let (_, changed) = draw(&context, 1.0, &mut custom, false, vec![egui::Event::PointerMoved(position), egui::Event::PointerButton {
                pos: position, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
            }]);
            assert!(!changed);
            assert!(!custom);
        }
    }

    #[test]
    fn ease_out_matches_cubic_bezier_samples_and_bounds() {
        for (progress, expected) in [(-1.0, 0.0), (0.0, 0.0), (0.0971875, 0.15625),
            (0.3425, 0.5), (0.6665625, 0.84375), (1.0, 1.0), (2.0, 1.0)] {
            close(ease_out(progress), expected);
        }
    }

    #[test]
    fn slide_starts_settled_animates_and_stops_after_two_hundred_ms() {
        let mut slide = Slide::settled(0.0, 1);
        close(slide.sample(0.0, 0.0, 1), 0.0);
        assert!(!slide.running(0.0));
        close(slide.sample(1.0, 1.0, 2), 0.0);
        close(slide.sample(1.0, 1.0685, 3), 0.5);
        assert!(slide.running(1.0685));
        close(slide.sample(1.0, 1.2, 4), 1.0);
        assert!(!slide.running(1.2));
    }

    #[test]
    fn reversal_is_continuous_and_shortens_the_return() {
        let mut slide = Slide::settled(0.0, 0);
        slide.sample(1.0, 1.0, 1);
        close(slide.sample(0.0, 1.0685, 2), 0.5);
        close(slide.shortening, 0.5);
        close(slide.sample(0.0, 1.10275, 3), 0.25);
        close(slide.sample(1.0, 1.10275, 3), 0.25);
        close(slide.shortening, 0.75);
        close(slide.sample(1.0, 1.154125, 4), 0.625);
        close(slide.sample(1.0, 1.26, 5), 1.0);
    }

    #[test]
    fn remount_settles_at_the_current_mode_instead_of_resuming_old_motion() {
        let mut slide = Slide::settled(0.0, 0);
        slide.sample(1.0, 1.0, 1);
        close(slide.sample(1.0, 1.03, 4), 1.0);
        assert!(!slide.running(1.03));
        close(slide.sample(0.0, 1.04, 7), 0.0);
        assert!(!slide.running(1.04));
    }
}
