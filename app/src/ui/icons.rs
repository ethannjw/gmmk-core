use eframe::egui::{self, Color32, Rect, Stroke};

#[derive(Clone, Copy)]
pub(super) enum Icon { Devices, Keyboard, Performance, Lighting, Keybinding, Settings }

pub(super) fn paint(ui: &egui::Ui, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.3_f32, color);
    let point = |horizontal: f32, vertical: f32| rect.min + egui::vec2(horizontal * rect.width() / 24.0, vertical * rect.height() / 24.0);
    let line = |points: &[(f32, f32)]| {
        ui.painter().add(egui::Shape::line(points.iter().map(|&(horizontal, vertical)| point(horizontal, vertical)).collect(), stroke));
    };
    match icon {
        Icon::Devices => {
            line(&[(3.0, 11.0), (12.0, 3.0), (21.0, 11.0), (21.0, 21.0), (15.0, 21.0), (15.0, 14.0),
                (9.0, 14.0), (9.0, 21.0), (3.0, 21.0), (3.0, 11.0)]);
        }
        Icon::Keyboard => {
            line(&[(2.0, 5.0), (22.0, 5.0), (22.0, 19.0), (2.0, 19.0), (2.0, 5.0)]);
            for row in [9.0, 12.0] {
                for column in [6.0, 10.0, 14.0, 18.0] { line(&[(column, row), (column + 1.0, row)]); }
            }
            line(&[(7.0, 16.0), (17.0, 16.0)]);
        }
        Icon::Performance => {
            line(&[(3.0, 19.0), (3.0, 21.0), (21.0, 21.0), (21.0, 19.0)]);
            line(&[(6.0, 17.0), (6.0, 13.0), (9.0, 13.0), (9.0, 17.0)]);
            line(&[(11.0, 17.0), (11.0, 9.0), (14.0, 9.0), (14.0, 17.0)]);
            line(&[(16.0, 17.0), (16.0, 4.0), (19.0, 4.0), (19.0, 17.0)]);
        }
        Icon::Lighting => {
            ui.painter().circle_stroke(point(12.0, 9.0), rect.width() * 0.24, stroke);
            line(&[(8.0, 13.0), (9.0, 18.0), (15.0, 18.0), (16.0, 13.0)]);
            line(&[(9.0, 21.0), (15.0, 21.0)]);
        }
        Icon::Keybinding => {
            line(&[(3.0, 6.0), (20.0, 6.0), (16.0, 2.0)]);
            line(&[(20.0, 6.0), (16.0, 10.0)]);
            line(&[(21.0, 18.0), (4.0, 18.0), (8.0, 14.0)]);
            line(&[(4.0, 18.0), (8.0, 22.0)]);
        }
        Icon::Settings => {
            ui.painter().circle_stroke(point(12.0, 12.0), rect.width() * 0.28, stroke);
            ui.painter().circle_stroke(point(12.0, 12.0), rect.width() * 0.10, stroke);
            for angle in 0..8 {
                let direction = egui::Vec2::angled(angle as f32 * std::f32::consts::FRAC_PI_4);
                ui.painter().line_segment([rect.center() + direction * rect.width() * 0.28,
                    rect.center() + direction * rect.width() * 0.43], stroke);
            }
        }
    }
}
