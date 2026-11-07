use super::*;
use crate::ui::appearance::{self, ACCENT, PANEL, RAIL, TEXT};
use egui::{Color32, FontId, Rect, RichText, Stroke};
use crate::ui::icons::{self, Icon};

impl GmmkApp {
    fn open_keyboard(&mut self) {
        if self.local_profile.is_none() {
            self.local_profile = Some(Self::local_defaults());
            self.onboard_slot = 0;
            self.edit_layer = 0;
            self.dirty = false;
            self.status_line = "Local factory defaults loaded. Onboard values remain unknown.".into();
        }
        self.select_page(Page::Home);
        self.screen = Screen::Keyboard;
    }

    fn local_defaults() -> ProfileFile {
        let mut profile = parse_profile_json(include_str!(
            "../../../assets/factory_profile.gmmkprofile.json"
        )).expect("embedded local defaults");
        for (index, slot) in profile.profiles.iter_mut().enumerate() {
            slot.name = format!("Profile {}", index + 1);
        }
        profile
    }

    pub(super) fn profiles(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Keyboard Profiles").size(16.0).family(appearance::heading_family()).color(Color32::WHITE));
        ui.add_space(12.0);
        ui.label(RichText::new("Manage and customize up to 3 profiles\nfor your Keyboard.").size(16.0));
        ui.add_space(4.0);
        for slot in 0..3 {
            let name = self.local_profile.as_ref().unwrap().profiles[slot].name.clone();
            let selected = self.onboard_slot as usize == slot;
            ui.push_id(("profile-row", slot), |ui| {
                let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 59.0), egui::Sense::hover());
                ui.painter().rect_filled(row, 0.0, PANEL);
                ui.painter().rect_stroke(row, 2.0, Stroke::new(1.0_f32,
                    if selected { Color32::from_rgb(91, 80, 57) } else { Color32::from_gray(51) }), egui::StrokeKind::Inside);
                let marker = row.left_center() + egui::vec2(20.0, 0.0);
                ui.painter().circle_stroke(marker, 7.5, Stroke::new(1.5_f32, if selected { ACCENT } else { TEXT }));
                if selected {
                    ui.painter().circle_filled(marker, 3.0, ACCENT);
                }
                let button_rect = Rect::from_min_max(row.min, row.right_bottom() - egui::vec2(48.0, 0.0));
                let response = ui.interact(button_rect, ui.id().with("select"), egui::Sense::click());
                let title = format!("Profile {}", slot + 1);
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &title));
                let custom_name = name != title;
                ui.painter().text(row.left_center() + egui::vec2(42.0, if custom_name { -8.0 } else { 0.0 }),
                    egui::Align2::LEFT_CENTER, &title, FontId::proportional(14.0), Color32::WHITE);
                if custom_name {
                    let galley = ui.painter().layout(name, FontId::proportional(11.0), TEXT, button_rect.width() - 60.0);
                    ui.painter().with_clip_rect(button_rect.shrink(3.0)).galley(row.left_center() + egui::vec2(42.0, 3.0), galley, TEXT);
                }
                if response.hovered() || response.has_focus() {
                    ui.painter().rect_stroke(button_rect, 0.0, Stroke::new(1.0_f32, TEXT), egui::StrokeKind::Inside);
                }
                if response.clicked() { self.select_slot(slot as u8); }
                let menu_rect = Rect::from_min_size(row.right_top() + egui::vec2(-46.0, 8.0), egui::vec2(42.0, 42.0));
                ui.scope_builder(egui::UiBuilder::new().max_rect(menu_rect), |ui| {
                    ui.menu_button("...", |ui| {
                        if ui.button("Rename").clicked() {
                            self.rename_profile = Some((slot, self.local_profile.as_ref().unwrap().profiles[slot].name.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Import profile…").on_hover_text("Import replaces all three local profiles after confirmation if edited.").clicked() {
                            ui.close_menu();
                            self.choose_import();
                        }
                        if ui.button("Export draft…").on_hover_text("Export all three local profiles to JSON.").clicked() {
                            ui.close_menu();
                            self.choose_export();
                        }
                        ui.separator();
                        if ui.button("Restore local defaults").clicked() {
                            self.restore_profile = Some(slot);
                            ui.close_menu();
                        }
                    }).response.on_hover_text(format!("Profile {} options", slot + 1));
                });
            });
        }
        ui.add_space(12.0);
        ui.label(RichText::new("Local profile · onboard values unknown").size(12.0).color(Color32::from_rgb(218, 189, 130)));
        ui.label(RichText::new("Selecting a profile does not activate it on the keyboard.").size(11.0));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Import profile…").clicked() { self.choose_import(); }
            if ui.button("Export draft…").clicked() { self.choose_export(); }
        });
    }

    pub(super) fn profile_dialogs(&mut self, context: &egui::Context) {
        if let Some((slot, mut name)) = self.rename_profile.clone() {
            egui::Window::new("Rename local profile").collapsible(false).resizable(false).show(context, |ui| {
                ui.add(egui::TextEdit::singleline(&mut name).char_limit(80));
                self.rename_profile = Some((slot, name.clone()));
                ui.horizontal(|ui| {
                    if ui.button("Cancel rename").clicked() { self.rename_profile = None; }
                    if ui.add_enabled(!name.trim().is_empty(), egui::Button::new("Save name")).clicked() {
                        if let Some(profile) = self.local_profile.as_mut().and_then(|file| file.profiles.get_mut(slot)) {
                            if profile.name != name.trim() {
                                profile.name = name.trim().to_owned();
                                self.dirty = true;
                            }
                        }
                        self.rename_profile = None;
                    }
                });
            });
        }
        if let Some(slot) = self.restore_profile {
            egui::Window::new("Restore local defaults?").collapsible(false).resizable(false).show(context, |ui| {
                ui.label(format!("Replace profile {} and its three layers with local factory defaults?", slot + 1));
                ui.label("Unexported changes in this profile will be lost. The keyboard is unchanged.");
                ui.horizontal(|ui| {
                    if ui.button("Keep profile").clicked() { self.restore_profile = None; }
                    if ui.button("Restore this local profile").clicked() {
                        if let Some(profile) = self.local_profile.as_mut().and_then(|file| file.profiles.get_mut(slot)) {
                            *profile = Self::local_defaults().profiles[slot].clone();
                            self.dirty = true;
                            self.sync_effect_page();
                            if let Some(index) = self.selected_key { self.select_key(index); }
                            self.status_line = format!("Local profile {} restored. Keyboard unchanged.", slot + 1);
                        }
                        self.restore_profile = None;
                    }
                });
            });
        }
    }

    pub(super) fn navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("G2").size(24.0).strong().color(Color32::WHITE));
            ui.add_space(22.0);
            let response = navigation_button(ui, "Devices", Icon::Devices, self.screen == Screen::Devices);
            if response.clicked() { self.screen = Screen::Devices; }
            if self.screen == Screen::Keyboard {
                ui.add_space(8.0);
                if navigation_button(ui, "GMMK 2", Icon::Keyboard, true).clicked() {
                    self.select_page(Page::Home);
                }
            }
        });
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(10.0);
            if navigation_button(ui, "Settings", Icon::Settings, self.tools_open).clicked() {
                self.tools_open = true;
            }
        });
    }

    pub(super) fn device_chooser(&mut self, ui: &mut egui::Ui) {
        let bounds = ui.max_rect();
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let connected = matches!(self.device.status(), DeviceStatus::Connected { .. });
            ui.add_space(if connected { (bounds.width() - 300.0) / 2.0 } else { 22.0 });
            match self.device.status() {
                DeviceStatus::Connected { .. } => {
                    let (card, response) = ui.allocate_exact_size(egui::vec2(300.0, 250.0), egui::Sense::click());
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "GMMK V2 96 ANSI"));
                    let title = Rect::from_min_size(card.min, egui::vec2(255.0, 40.0));
                    let swatch = Rect::from_min_size(card.min + egui::vec2(260.0, 0.0), egui::vec2(40.0, 40.0));
                    let image = Rect::from_min_max(card.min + egui::vec2(0.0, 45.0), card.max);
                    ui.painter().rect_filled(title, 0.0, RAIL);
                    ui.painter().rect_filled(swatch, 0.0, RAIL);
                    ui.painter().circle(swatch.center(), 10.0, Color32::from_gray(23), Stroke::new(1.0_f32, TEXT));
                    ui.painter().rect_filled(image, 0.0, RAIL);
                    ui.painter().text(title.left_center() + egui::vec2(18.0, 0.0), egui::Align2::LEFT_CENTER,
                        "GMMK V2 96 ANSI", FontId::proportional(14.0), Color32::WHITE);
                    appearance::keyboard(ui, image.shrink(5.0), &self.layout, None, None, false);
                    if response.hovered() || response.has_focus() {
                        ui.painter().rect_stroke(card, 0.0, Stroke::new(1.0_f32, ACCENT), egui::StrokeKind::Inside);
                    }
                    if response.clicked() { self.open_keyboard(); }
                }
                status => {
                    ui.vertical(|ui| {
                        ui.set_max_width(560.0);
                        let title = if matches!(status, DeviceStatus::Error(_)) {
                            "Keyboard access unavailable"
                        } else { "No supported keyboard connected" };
                        ui.label(RichText::new(title).size(18.0).color(Color32::WHITE));
                        ui.label("Connect your GMMK 2 96% ANSI by USB. Detection is automatic while connected; manual Disconnect pauses it.");
                        if let DeviceStatus::Error(error) = status { ui.label(error); }
                        ui.label("Only 320f:504b is supported. No onboard settings are downloaded or changed.");
                        if ui.add_enabled(!self.worker_stopped, egui::Button::new("Retry connection")).clicked() {
                            self.device.connect();
                        }
                    });
                }
            }
        });
        let footer = Rect::from_min_max(egui::pos2(bounds.left() + 22.0, bounds.bottom() - 125.0), bounds.right_bottom() - egui::vec2(180.0, 0.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(footer), |ui| {
            ui.label(RichText::new("Select your device").size(12.0));
            ui.horizontal(|ui| {
                if self.local_profile.is_some() && ui.button("Continue local draft").clicked() { self.open_keyboard(); }
                if ui.button("Import profile…").clicked() { self.choose_import(); }
            });
            ui.label(RichText::new(&self.status_line).size(12.0));
        });
        ui.painter().text(bounds.right_bottom() - egui::vec2(25.0, 40.0), egui::Align2::RIGHT_BOTTOM, "GMMK",
            FontId::new(20.0, appearance::heading_family()), Color32::from_gray(119));
        ui.painter().text(bounds.right_bottom() - egui::vec2(25.0, 22.0), egui::Align2::RIGHT_BOTTOM, "CONFIG",
            FontId::proportional(10.0), Color32::from_gray(119));
    }
}

fn navigation_button(ui: &mut egui::Ui, label: &str, icon: Icon, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(64.0, 64.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    let color = if selected || response.hovered() || response.has_focus() { ACCENT } else { TEXT };
    icons::paint(ui, Rect::from_center_size(rect.center() - egui::vec2(0.0, 8.0), egui::vec2(26.0, 26.0)), icon, color);
    ui.painter().text(rect.center() + egui::vec2(0.0, 19.0), egui::Align2::CENTER_CENTER, label,
        FontId::proportional(10.0), color);
    response
}
