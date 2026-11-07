use egui::{Color32, Id, Key, Painter, Rect, Response, Sense, Stroke, Ui};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Hsv {
    hue: f64,
    saturation: f64,
    value: f64,
}

impl Hsv {
    fn from_rgb(rgb: [u8; 3]) -> Self {
        let [red, green, blue] = rgb.map(f64::from);
        let maximum = red.max(green).max(blue);
        let difference = maximum - red.min(green).min(blue);
        let sector = if difference == 0.0 { 0.0 }
            else if maximum == red { (green - blue) / difference }
            else if maximum == green { 2.0 + (blue - red) / difference }
            else { 4.0 + (red - green) / difference };
        Self {
            hue: (60.0 * if sector < 0.0 { sector + 6.0 } else { sector }).round(),
            saturation: if maximum == 0.0 { 0.0 } else { (difference / maximum * 100.0).round() },
            value: (maximum / 255.0 * 100.0).round(),
        }
    }

    fn rgb(self) -> [u8; 3] {
        let hue = self.hue / 360.0 * 6.0;
        let saturation = self.saturation / 100.0;
        let value = self.value / 100.0;
        let sector = hue.floor();
        let minimum = value * (1.0 - saturation);
        let descending = value * (1.0 - (hue - sector) * saturation);
        let ascending = value * (1.0 - (1.0 - hue + sector) * saturation);
        let index = sector as usize % 6;
        [
            [value, descending, minimum, minimum, ascending, value][index],
            [ascending, value, value, descending, minimum, minimum][index],
            [minimum, minimum, ascending, value, value, descending][index],
        ].map(|channel| (255.0 * channel).round() as u8)
    }

    fn pure_hue(self) -> Color32 {
        let [red, green, blue] = Self { hue: self.hue, saturation: 100.0, value: 100.0 }.rgb();
        Color32::from_rgb(red, green, blue)
    }
}

#[derive(Clone, Copy)]
struct State {
    observed: [u8; 3],
    hsv: Hsv,
}

pub(super) fn show(ui: &mut Ui, scope: Id, rgb: &mut [u8; 3]) -> bool {
    let original = *rgb;
    let mut state = ui.ctx().data_mut(|data| data.get_temp::<State>(scope))
        .filter(|state| state.observed == original)
        .unwrap_or(State { observed: original, hsv: Hsv::from_rgb(original) });
    let before = state.hsv;
    let (area, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 200.0), Sense::hover());
    let saturation = Rect::from_min_size(area.min, egui::vec2(area.width(), 164.0));
    let hue = Rect::from_min_max(area.left_top() + egui::vec2(0.0, 176.0), area.right_bottom());
    let saturation_response = ui.interact(saturation, scope.with("saturation"), Sense::click_and_drag());
    let hue_response = ui.interact(hue, scope.with("hue"), Sense::click_and_drag());
    saturation_response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, ui.is_enabled(),
        format!("Color: saturation {:.0}%, brightness {:.0}%", state.hsv.saturation, state.hsv.value)));
    hue_response.widget_info(|| egui::WidgetInfo::slider(ui.is_enabled(), state.hsv.hue, "Hue"));
    if let Some(position) = pointer(&saturation_response) {
        state.hsv.saturation = f64::from(((position.x - saturation.left()) / saturation.width()).clamp(0.0, 1.0)) * 100.0;
        state.hsv.value = 100.0 - f64::from(((position.y - saturation.top()) / saturation.height()).clamp(0.0, 1.0)) * 100.0;
    }
    if let Some(position) = pointer(&hue_response) {
        state.hsv.hue = f64::from(((position.x - hue.left()) / hue.width()).clamp(0.0, 1.0)) * 360.0;
    }
    for response in [&saturation_response, &hue_response] {
        if response.has_focus() {
            ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, egui::EventFilter {
                horizontal_arrows: true, vertical_arrows: true, ..Default::default()
            }));
        }
    }
    for (key, horizontal, vertical) in [(Key::ArrowLeft, -1.0, 0.0), (Key::ArrowRight, 1.0, 0.0),
        (Key::ArrowUp, 0.0, 1.0), (Key::ArrowDown, 0.0, -1.0)] {
        if (saturation_response.has_focus() || hue_response.has_focus()) &&
            ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key)) {
            if saturation_response.has_focus() {
                state.hsv.saturation = (state.hsv.saturation + horizontal * 5.0).clamp(0.0, 100.0);
                state.hsv.value = (state.hsv.value + vertical * 5.0).clamp(0.0, 100.0);
            } else {
                state.hsv.hue = (state.hsv.hue + horizontal * 18.0).clamp(0.0, 360.0);
            }
        }
    }
    if before != state.hsv {
        *rgb = state.hsv.rgb();
        state.observed = *rgb;
    }
    paint(ui.painter(), saturation, hue, state.hsv, saturation_response.has_focus(), hue_response.has_focus());
    ui.ctx().data_mut(|data| data.insert_temp(scope, state));
    *rgb != original
}

fn pointer(response: &Response) -> Option<egui::Pos2> {
    if response.is_pointer_button_down_on() || response.dragged() {
        response.request_focus();
        response.interact_pointer_pos()
    } else { None }
}

fn gradient(painter: &Painter, rect: Rect, colors: [Color32; 4]) {
    let mut mesh = egui::Mesh::default();
    for (position, color) in [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()].into_iter().zip(colors) {
        mesh.colored_vertex(position, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn paint(painter: &Painter, saturation: Rect, hue: Rect, hsv: Hsv, saturation_focused: bool, hue_focused: bool) {
    let pure = hsv.pure_hue();
    gradient(painter, saturation, [Color32::WHITE, pure, pure, Color32::WHITE]);
    gradient(painter, saturation, [Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK]);
    painter.rect_filled(Rect::from_min_max(saturation.left_bottom(), hue.right_top()), 0.0, Color32::BLACK);
    let stops = [(0.0, Color32::RED), (0.17, Color32::YELLOW), (0.33, Color32::GREEN),
        (0.5, Color32::from_rgb(0, 255, 255)), (0.67, Color32::BLUE), (0.83, Color32::from_rgb(255, 0, 255)), (1.0, Color32::RED)];
    for pair in stops.windows(2) {
        let rect = Rect::from_min_max(hue.left_top() + egui::vec2(hue.width() * pair[0].0, 0.0),
            hue.left_bottom() + egui::vec2(hue.width() * pair[1].0, 0.0));
        gradient(painter, rect, [pair[0].1, pair[1].1, pair[1].1, pair[0].1]);
    }
    let hue_center = hue.left_center() + egui::vec2(hue.width() * (hsv.hue / 360.0) as f32, 0.0);
    let hue_size = egui::vec2(7.0, 28.0) * if hue_focused { 1.1 } else { 1.0 };
    painter.rect(Rect::from_center_size(hue_center, hue_size), 3.0, pure,
        Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);
    let position = saturation.left_top() + egui::vec2(saturation.width() * (hsv.saturation / 100.0) as f32,
        saturation.height() * (1.0 - hsv.value / 100.0) as f32);
    let [red, green, blue] = hsv.rgb();
    let radius = if saturation_focused { 15.4 } else { 14.0 };
    painter.circle_filled(position, radius, Color32::WHITE);
    painter.circle_filled(position, radius - 2.0, Color32::from_rgb(red, green, blue));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Vector { rgb: [u8; 3], hsv: [f64; 3] }

    #[derive(serde::Deserialize)]
    struct Vectors { rgb_imports: Vec<Vector>, hsv_edits: Vec<Vector> }

    #[test]
    fn color_math_matches_shared_windows_vectors_without_linear_gamma_conversion() {
        let vectors: Vectors = serde_json::from_str(include_str!("../../assets/windows-color-surface-vectors.json")).unwrap();
        for entry in vectors.rgb_imports {
            let result = Hsv::from_rgb(entry.rgb);
            assert_eq!([result.hue, result.saturation, result.value], entry.hsv);
        }
        for entry in vectors.hsv_edits {
            let [hue, saturation, value] = entry.hsv;
            assert_eq!(Hsv { hue, saturation, value }.rgb(), entry.rgb);
        }
    }

    fn frame(context: &egui::Context, scope: Id, rgb: &mut [u8; 3], events: Vec<egui::Event>) -> bool {
        let mut changed = false;
        let _ = context.run(egui::RawInput { events, screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 400.0))),
            ..Default::default() }, |context| {
            egui::CentralPanel::default().show(context, |ui| { changed = show(ui, scope, rgb); });
        });
        changed
    }

    fn key(context: &egui::Context, scope: Id, rgb: &mut [u8; 3], key: Key) -> bool {
        frame(context, scope, rgb, vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false,
            modifiers: Default::default() }])
    }

    #[test]
    fn keyboard_steps_and_bounds_preserve_hue_at_black_without_mutating_on_render() {
        let context = egui::Context::default();
        let scope = Id::new("keyboard-color");
        let mut rgb = [0, 0, 0];
        assert!(!frame(&context, scope, &mut rgb, vec![]));
        context.memory_mut(|memory| memory.request_focus(scope.with("hue")));
        for _ in 0..30 { key(&context, scope, &mut rgb, Key::ArrowRight); }
        assert_eq!(rgb, [0, 0, 0]);
        assert_eq!(context.data_mut(|data| data.get_temp::<State>(scope)).unwrap().hsv.hue, 360.0);
        for _ in 0..10 { key(&context, scope, &mut rgb, Key::ArrowLeft); }
        context.memory_mut(|memory| memory.request_focus(scope.with("saturation")));
        for _ in 0..30 { key(&context, scope, &mut rgb, Key::ArrowRight); }
        for _ in 0..30 { key(&context, scope, &mut rgb, Key::ArrowUp); }
        assert_eq!(rgb, [0, 255, 255]);
        assert!(!frame(&context, scope, &mut rgb, vec![]));
        for _ in 0..30 { key(&context, scope, &mut rgb, Key::ArrowLeft); }
        assert_eq!(rgb, [255, 255, 255]);
        for _ in 0..30 { key(&context, scope, &mut rgb, Key::ArrowDown); }
        assert_eq!(rgb, [0, 0, 0]);
        key(&context, scope, &mut rgb, Key::Tab);
        assert!(!context.memory(|memory| memory.has_focus(scope.with("saturation"))));
    }

    #[test]
    fn external_rgb_replaces_cached_color_but_independent_scopes_keep_their_own_hue() {
        let context = egui::Context::default();
        let first = Id::new("first-color");
        let second = Id::new("second-color");
        let mut rgb = [255, 0, 0];
        frame(&context, first, &mut rgb, vec![]);
        context.memory_mut(|memory| memory.request_focus(first.with("hue")));
        key(&context, first, &mut rgb, Key::ArrowRight);
        assert_eq!(rgb, [255, 77, 0]);
        let mut other_rgb = [0, 0, 255];
        frame(&context, second, &mut other_rgb, vec![]);
        let cache = context.data_mut(|data| data.get_temp::<State>(first)).unwrap();
        assert_eq!(cache.hsv.hue, 18.0);
        rgb = [79, 22, 249];
        assert!(!frame(&context, first, &mut rgb, vec![]));
        assert_eq!(rgb, [79, 22, 249]);
        assert_eq!(context.data_mut(|data| data.get_temp::<State>(first)).unwrap().hsv, Hsv::from_rgb(rgb));
        assert_eq!(context.data_mut(|data| data.get_temp::<State>(second)).unwrap().hsv.hue, 240.0);
    }

    #[test]
    fn dragging_outside_the_surface_clamps_without_changing_the_other_hsv_axes() {
        let context = egui::Context::default();
        let scope = Id::new("drag-color");
        let mut rgb = [255, 0, 0];
        frame(&context, scope, &mut rgb, vec![]);
        frame(&context, scope, &mut rgb, vec![]);
        let rect = context.read_response(scope.with("saturation")).unwrap().rect;
        frame(&context, scope, &mut rgb, vec![egui::Event::PointerMoved(rect.center()), egui::Event::PointerButton {
            pos: rect.center(), button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default(),
        }]);
        assert_eq!(rgb, [128, 64, 64]);
        for (position, expected) in [(rect.right_top() + egui::vec2(30.0, -30.0), [255, 0, 0]),
            (rect.left_bottom() + egui::vec2(-30.0, 30.0), [0, 0, 0])] {
            frame(&context, scope, &mut rgb, vec![egui::Event::PointerMoved(position)]);
            assert_eq!(rgb, expected);
        }
    }

    #[test]
    fn surface_draws_separate_white_hue_and_black_gradients_with_reference_pointer_sizes() {
        let context = egui::Context::default();
        let scope = Id::new("color-shapes");
        let mut rgb = [0, 255, 0];
        let output = context.run(egui::RawInput::default(), |context| {
            egui::CentralPanel::default().show(context, |ui| { assert!(!show(ui, scope, &mut rgb)); });
        });
        let meshes: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) => Some(mesh), _ => None,
        }).collect();
        assert_eq!(meshes.len(), 8);
        assert_eq!(meshes[0].vertices.iter().map(|vertex| vertex.color).collect::<Vec<_>>(),
            vec![Color32::WHITE, Color32::GREEN, Color32::GREEN, Color32::WHITE]);
        assert_eq!(meshes[1].vertices.iter().map(|vertex| vertex.color).collect::<Vec<_>>(),
            vec![Color32::TRANSPARENT, Color32::TRANSPARENT, Color32::BLACK, Color32::BLACK]);
        assert_eq!(meshes[0].vertices[3].pos.y - meshes[0].vertices[0].pos.y, 164.0);
        let stops = [Color32::RED, Color32::YELLOW, Color32::GREEN, Color32::from_rgb(0, 255, 255),
            Color32::BLUE, Color32::from_rgb(255, 0, 255), Color32::RED];
        for (index, mesh) in meshes[2..].iter().enumerate() {
            assert_eq!(mesh.vertices[0].color, stops[index]);
            assert_eq!(mesh.vertices[1].color, stops[index + 1]);
            assert_eq!(mesh.vertices[3].pos.y - mesh.vertices[0].pos.y, 24.0);
        }
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.rect.size() == egui::vec2(7.0, 28.0) && rect.corner_radius == egui::CornerRadius::same(3))));
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Circle(circle) if circle.radius == 14.0 && circle.fill == Color32::WHITE)));
    }
}
