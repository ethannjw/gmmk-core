use super::*;
use crate::ui::appearance::{self, ACCENT, BACKGROUND, DEVICE_TAB_ACCENT, PANEL, RAIL, TEXT};
use egui::{Color32, Rect, Stroke};

impl GmmkApp {
    pub(super) fn show(&mut self, context: &egui::Context) {
        self.poll_device();
        context.request_repaint_after(std::time::Duration::from_millis(100));
        egui::TopBottomPanel::top("title-bar").exact_height(25.0)
            .frame(egui::Frame::new().fill(RAIL).inner_margin(5.0)).show(context, |ui| {
                ui.small("GMMK Config");
            });
        egui::SidePanel::left("navigation").exact_width(80.0).resizable(false)
            .frame(egui::Frame::new().fill(RAIL).inner_margin(8.0)).show(context, |ui| {
                self.navigation(ui);
            });
        let background = if self.screen == Screen::Devices { Color32::from_gray(48) } else { BACKGROUND };
        egui::CentralPanel::default().frame(egui::Frame::new().fill(background).inner_margin(10.0))
            .show(context, |ui| {
                if self.screen == Screen::Devices {
                    self.device_chooser(ui);
                    return;
                }
                let bounds = ui.max_rect();
                let header = Rect::from_min_size(bounds.min, egui::vec2(bounds.width(), 142.0));
                self.header(ui, header);
                let status_height = ui.painter().layout(
                    self.pending_write.as_ref().unwrap_or(&self.status_line).clone(),
                    egui::TextStyle::Body.resolve(ui.style()), TEXT,
                    (bounds.width() - 240.0).max(200.0),
                ).size().y;
                let footer_height = (status_height + 76.0).max(90.0);
                let editor = Rect::from_min_max(
                    egui::pos2(bounds.left() + 20.0, header.bottom() + if self.page == Page::Home { 34.0 } else { 24.0 }),
                    egui::pos2(bounds.right() - 10.0, bounds.bottom() - footer_height - 10.0),
                );
                self.editor(ui, editor);
                let footer = Rect::from_min_max(egui::pos2(bounds.left() + 20.0, bounds.bottom() - footer_height), bounds.max);
                self.footer(ui, footer);
            });
        self.device_tools(context);
        self.import_confirmation(context);
        self.profile_dialogs(context);
    }

    fn header(&mut self, ui: &mut egui::Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0.0, PANEL);
        let back = Rect::from_min_size(rect.min + egui::vec2(16.0, 16.0), egui::vec2(30.0, 38.0));
        let response = ui.interact(back, ui.id().with("back-to-devices"), egui::Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Back to devices"));
        ui.painter().add(egui::Shape::line(vec![back.center() + egui::vec2(3.0, -7.0),
            back.center() - egui::vec2(3.0, 0.0), back.center() + egui::vec2(3.0, 7.0)], Stroke::new(1.5_f32, TEXT)));
        if response.on_hover_text("Back to devices").clicked() { self.screen = Screen::Devices; }
        ui.painter().with_clip_rect(rect).text(egui::pos2(rect.right() - 170.0, rect.top() - 50.0), egui::Align2::RIGHT_TOP,
            "G2", egui::FontId::new(220.0, appearance::heading_family()), Color32::from_gray(44));
        ui.painter().text(rect.min + egui::vec2(58.0, 35.0), egui::Align2::LEFT_CENTER,
            "GMMK V2 96 ANSI", egui::FontId::new(20.0, appearance::heading_family()), Color32::WHITE);
        let connected = matches!(self.device.status(), DeviceStatus::Connected { .. });
        let state = if connected { "Connected" } else { "Disconnected" };
        let firmware = self.firmware.as_deref().unwrap_or("unknown");
        let status = format!("{state}  ·  Firmware (USB) {firmware}");
        if ui.put(Rect::from_min_size(egui::pos2(rect.right() - 300.0, rect.top() + 16.0), egui::vec2(280.0, 40.0)),
            egui::Button::new(status).corner_radius(20.0)).clicked() {
            self.tools_open = true;
        }
        for (index, page) in Page::ALL.iter().enumerate() {
            let tab = Rect::from_min_size(rect.min + egui::vec2(12.0 + index as f32 * 128.0, 100.0), egui::vec2(126.0, 40.0));
            let selected = self.page == *page;
            let color = if selected { DEVICE_TAB_ACCENT } else { TEXT };
            let response = ui.interact(tab, ui.id().with(("device-tab", index)), egui::Sense::click());
            response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, page.label()));
            let icon = match page {
                Page::Home => crate::ui::icons::Icon::Keyboard,
                Page::Performance => crate::ui::icons::Icon::Performance,
                Page::Lighting => crate::ui::icons::Icon::Lighting,
                Page::Keybinding => crate::ui::icons::Icon::Keybinding,
            };
            crate::ui::icons::paint(ui, Rect::from_center_size(tab.left_center() + egui::vec2(14.0, 0.0), egui::vec2(21.0, 21.0)), icon, color);
            ui.painter().text(tab.left_center() + egui::vec2(32.0, 0.0), egui::Align2::LEFT_CENTER,
                page.label(), egui::FontId::proportional(12.0), color);
            if response.clicked() {
                self.select_page(*page);
            }
            if selected {
                ui.painter().line_segment([tab.left_bottom(), tab.right_bottom()], Stroke::new(2.0_f32, DEVICE_TAB_ACCENT));
            }
        }
    }

    fn import_screen(&mut self, ui: &mut egui::Ui, rect: Rect) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            appearance::heading(ui, "Import a local profile");
            ui.label("Connect detects the keyboard without changing its settings.");
            ui.label(ONBOARD_READ_UNAVAILABLE);
            ui.label("Import your saved JSON profile to begin. The factory template is not an onboard backup.");
            if ui.button("Import profile…").clicked() { self.choose_import(); }
        });
    }

    fn editor(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let has_profile = self.local_profile.is_some();
        let minimum_width = if self.page == Page::Lighting && !self.custom_lighting { crate::ui::preset_effects::WIDTH + 12.0 } else { 290.0 };
        let control_width = (rect.width() * 0.32).clamp(minimum_width, 400.0);
        let controls = Rect::from_min_size(rect.min, egui::vec2(control_width, rect.height()));
        let preview = Rect::from_min_max(egui::pos2(controls.right() + 30.0, rect.top()), rect.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(controls), |ui| {
            egui::ScrollArea::vertical().id_salt(("controls", self.page.label())).show(ui, |ui| {
                ui.set_width(if self.page == Page::Home { control_width } else { control_width - 12.0 });
                if has_profile {
                    match self.page {
                        Page::Home => self.profiles(ui),
                        Page::Performance => self.performance(ui),
                        Page::Lighting => self.lighting(ui),
                        Page::Keybinding => self.bindings(ui),
                    }
                } else {
                    self.import_screen(ui, ui.available_rect_before_wrap());
                }
            });
        });
        let mut board_area = preview;
        if self.page == Page::Home { board_area.min.y += 100.0; }
        if !has_profile {
            ui.put(Rect::from_min_size(preview.min, egui::vec2(preview.width(), 40.0)),
                egui::Label::new("Keyboard layout preview — onboard settings unknown."));
            board_area.min.y += 48.0;
        } else if self.page == Page::Keybinding {
            let mut layer = self.edit_layer;
            let changed = ui.scope_builder(egui::UiBuilder::new().max_rect(
                Rect::from_min_size(preview.min, egui::vec2(preview.width(), 90.0))), |ui| {
                crate::ui::keyboard_layers::show(ui, &mut layer)
            }).inner;
            if changed { self.select_layer(layer); }
            board_area.min.y += 90.0;
        } else if self.page != Page::Home {
            ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_size(preview.min, egui::vec2(preview.width(), 90.0))), |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Local profile {}", self.onboard_slot + 1));
                    ui.label("Keyboard Layer");
                });
                ui.horizontal(|ui| {
                    for layer in 0..3 {
                        if appearance::choice(ui, &format!("Layer {}", layer + 1), self.edit_layer == layer).clicked() {
                            self.select_layer(layer);
                        }
                    }
                });
            });
            board_area.min.y += 80.0;
        }
        let paint = self.page == Page::Lighting && self.custom_lighting;
        if has_profile && paint {
            let quick_area = Rect::from_min_max(
                egui::pos2(preview.left(), preview.bottom() - crate::ui::quick_keys::HEIGHT), preview.max,
            );
            board_area.max.y = quick_area.top() - 12.0;
            let enabled = self.erase_lighting || self.paint_color().is_some();
            let selection = ui.scope_builder(egui::UiBuilder::new().max_rect(quick_area), |ui| {
                crate::ui::quick_keys::show(ui, enabled)
            }).inner;
            match selection {
                Some(crate::ui::quick_keys::Selection::Group(index)) => self.paint_keys(LIGHTING_KEY_GROUPS[index].1),
                Some(crate::ui::quick_keys::Selection::All) => self.paint_all(),
                None => {},
            }
        }
        let lighting = self.current_layer().map(|layer| &layer.lighting);
        let interactive = has_profile && (self.page == Page::Keybinding || (paint && (self.erase_lighting || self.paint_color().is_some())));
        if let Some(index) = appearance::keyboard(ui, board_area, &self.layout,
            if self.page == Page::Keybinding { self.selected_key } else { None }, lighting, interactive) {
            if paint {
                if let Some(key) = self.layout.device.matrix_key_buttons.get(index).cloned() {
                    self.paint_keys(&[&key]);
                }
            } else {
                self.select_key(index);
            }
        }
    }

    fn performance(&mut self, ui: &mut egui::Ui) {
        appearance::heading(ui, "General Performance");
        ui.label("Customize the performance settings in your local draft.");
        ui.add_space(12.0);
        let mut changed = false;
        if let Some(layer) = self.current_layer_mut() {
            egui::Frame::new().fill(PANEL).corner_radius(5.0).inner_margin(18.0).show(ui, |ui| {
                ui.set_min_width(240.0);
                ui.label("Polling Rate");
                egui::ComboBox::from_id_salt("polling").selected_text(format!("{} Hz", layer.polling_hz)).show_ui(ui, |ui| {
                    for rate in [125, 250, 500, 1000] {
                        changed |= ui.selectable_value(&mut layer.polling_hz, rate, format!("{rate} Hz")).changed();
                    }
                });
            });
            egui::Frame::new().fill(PANEL).corner_radius(5.0).inner_margin(18.0).show(ui, |ui| {
                ui.set_min_width(240.0);
                ui.label("Input Latency");
                egui::ComboBox::from_id_salt("latency").selected_text(format!("{} ms", layer.lighting.input_latency_ms)).show_ui(ui, |ui| {
                    for latency in [2, 8, 16] {
                        changed |= ui.selectable_value(&mut layer.lighting.input_latency_ms, latency, format!("{latency} ms")).changed();
                    }
                });
            });
        }
        self.dirty |= changed;
        egui::Frame::new().fill(PANEL).corner_radius(5.0).inner_margin(18.0).show(ui, |ui| {
            ui.set_min_width(240.0);
            ui.label("Operating System");
            ui.add_enabled_ui(false, |ui| {
                ui.horizontal(|ui| { let _ = ui.button("Windows"); let _ = ui.button("macOS"); });
            });
            ui.small("Device OS-mode behavior is not verified.");
        });
    }

    fn lighting(&mut self, ui: &mut egui::Ui) {
        let spacing = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let mut custom = self.custom_lighting;
        if crate::ui::lighting_mode::show(ui, &mut custom) { self.select_lighting_mode(custom); }
        ui.add_space(20.0);
        ui.spacing_mut().item_spacing.y = spacing;
        if self.custom_lighting {
            appearance::heading(ui, "Custom Per Key");
            ui.label("Click a key or quick group to paint the local draft.");
            let mut color = self.paint_color().unwrap_or([255, 0, 0]);
            ui.horizontal(|ui| {
                ui.label("RGB");
                if ui.color_edit_button_srgb(&mut color).changed() {
                    self.paint_hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
                }
            });
            ui.horizontal(|ui| {
                ui.label("HEX");
                ui.add(egui::TextEdit::singleline(&mut self.paint_hex).desired_width(100.0));
            });
            let mut color = self.paint_color().unwrap_or([255, 0, 0]);
            if crate::ui::color_surface::show(ui, egui::Id::new(("custom-color", self.onboard_slot, self.edit_layer)), &mut color) {
                self.paint_hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
            }
            let valid_color = self.paint_color().is_some();
            if !valid_color { ui.colored_label(ACCENT, "Enter six hex digits, for example #FF0000."); }
            crate::ui::breathing_toggle::show(ui, &mut self.paint_breathing);
            ui.horizontal(|ui| {
                if appearance::choice(ui, "Add", !self.erase_lighting).clicked() { self.erase_lighting = false; }
                if appearance::choice(ui, "Erase", self.erase_lighting).clicked() { self.erase_lighting = true; }
            });
            if ui.button("Clear custom lighting").on_hover_text("Stages four empty pages. Save to Keyboard clears onboard custom colors.").clicked() {
                self.current_layer_mut().unwrap().lighting.per_key_colors = Some(Vec::new());
                self.dirty = true;
            }
            if ui.button("Remove local override data").on_hover_text("Leaves onboard per-key data untouched when saving.").clicked() {
                self.current_layer_mut().unwrap().lighting.per_key_colors = None;
                self.dirty = true;
            }
            ui.small("All includes 20 side LEDs. Separate side-light selection/preview, saved colors and per-key brightness remain unavailable.");
            return;
        }
        let mut changed = false;
        if let Some(effect) = self.current_layer().map(|layer| layer.lighting.effect_ui_value) {
            let mut selected = effect;
            changed = crate::ui::preset_effects::show(ui, &mut selected, &mut self.effect_page);
            if changed { self.current_layer_mut().unwrap().lighting.effect_ui_value = selected; }
        }
        let scope = (self.onboard_slot, self.edit_layer);
        let mut color_changed = false;
        if let Some(layer) = self.current_layer_mut() {
            let effect = layer.lighting.effect_ui_value;
            if !matches!(effect, 0 | 5 | 8 | 13) {
                ui.label("Effect Color");
                color_changed |= ui.color_edit_button_srgb(&mut layer.lighting.rgb).changed();
                color_changed |= crate::ui::color_surface::show(ui, egui::Id::new(("preset-color", scope.0, scope.1)), &mut layer.lighting.rgb);
            }
            if effect != 8 {
                ui.label("Brightness");
                changed |= crate::ui::preset_slider::show(ui, (scope, "brightness"), &mut layer.lighting.brightness, 0);
            }
            if !matches!(effect, 6 | 8) {
                ui.label("Speed");
                changed |= crate::ui::preset_slider::show(ui, (scope, "speed"), &mut layer.lighting.speed, 1);
            }
        }
        if color_changed { self.sync_paint_color(); }
        self.dirty |= changed || color_changed;
    }

    fn bindings(&mut self, ui: &mut egui::Ui) {
        appearance::heading(ui, "Key Binding");
        ui.label("Choose a function to map to the selected key.");
        ui.add_space(12.0);
        let Some(index) = self.selected_key else {
            ui.label("Select a key on the keyboard to enable binding options.");
            return;
        };
        ui.horizontal(|ui| {
            if appearance::choice(ui, "Keyboard Function", self.binding_category == BindingCategory::Keyboard).clicked() {
                self.binding_category = BindingCategory::Keyboard;
            }
            if appearance::choice(ui, "Multimedia", self.binding_category == BindingCategory::Multimedia).clicked() {
                self.binding_category = BindingCategory::Multimedia;
            }
        });
        ui.label(format!("Selected: {}", self.layout.device.matrix_key_buttons[index].trim()));
        let action = decode_key_action(&self.current_layer().unwrap().key_matrix, &self.layout, index);
        match action {
            KeyAction::Default => { ui.small("Imported action is not supported by this editor. Assigning replaces it explicitly."); }
            KeyAction::Disabled => { ui.small("This key is disabled in the draft."); }
            _ => {}
        }
        if self.binding_category == BindingCategory::Multimedia {
            ui.label("Choose a media action to assign it to the local draft.");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                for media in &MediaAction::ALL {
                    if appearance::binding_tile(ui, media.label(), action == KeyAction::Multimedia(*media)).clicked() {
                        self.assign_selected(KeyAction::Multimedia(*media));
                    }
                }
            });
            ui.small("OS/player behavior requires hardware acceptance on each platform.");
        } else {
            egui::ComboBox::from_label("Category").selected_text(self.keyboard_category.label()).show_ui(ui, |ui| {
                for category in KeyboardCategory::ALL {
                    ui.selectable_value(&mut self.keyboard_category, category, category.label());
                }
            });
            let choice = self.keyboard_category.choice_for_code(self.key_picker_hid);
            let name = choice.map(|choice| choice.label).unwrap_or("Select key");
            let mut selected_code = None;
            ui.push_id(self.keyboard_category.label(), |ui| {
                egui::ComboBox::from_label("Key").selected_text(name).show_ui(ui, |ui| {
                    for choice in self.keyboard_category.choices() {
                        ui.push_id(choice.id, |ui| {
                            if let Some(code) = choice.code {
                                if ui.selectable_value(&mut self.key_picker_hid, code, choice.label).clicked() {
                                    selected_code = Some(code);
                                }
                            } else {
                                ui.add_enabled(false, egui::SelectableLabel::new(false, choice.label))
                                    .on_disabled_hover_text(format!("{}: the Windows device mapping is unresolved; assignment is unavailable.", choice.id));
                            }
                        });
                    }
                });
            });
            if let Some(code) = selected_code {
                self.key_picker_modifiers = (false, false, false, false);
                self.assign_selected(KeyAction::HidKey { code, ctrl: false, shift: false, alt: false, win: false });
            }
            if let KeyAction::HidKey { code, .. } = action {
                if KeyboardCategory::for_code(code).is_none() {
                    ui.small(format!("Imported HID {code} is preserved. Select a verified key to replace it."));
                }
            }
            let mut modifier_changed = false;
            if ui.checkbox(&mut self.include_key_modifier, "Include Modifier").changed() && !self.include_key_modifier {
                self.key_picker_modifiers = (false, false, false, false);
                modifier_changed = true;
            }
            if self.include_key_modifier {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    for (label, modifiers) in [
                        ("Shift", (false, true, false, false)),
                        ("Ctrl", (true, false, false, false)),
                        ("Alt", (false, false, true, false)),
                        ("Meta", (false, false, false, true)),
                    ] {
                        if appearance::binding_tile(ui, label, self.key_picker_modifiers == modifiers).clicked() {
                            self.key_picker_modifiers = modifiers;
                            modifier_changed = true;
                        }
                    }
                });
                let (ctrl, shift, alt, win) = self.key_picker_modifiers;
                if [ctrl, shift, alt, win].into_iter().filter(|enabled| *enabled).count() > 1 {
                    ui.small("Imported combined modifiers are preserved until you choose a modifier.");
                }
            }
            if modifier_changed {
                if let KeyAction::HidKey { code, .. } = decode_key_action(&self.current_layer().unwrap().key_matrix, &self.layout, index) {
                    if KeyboardCategory::for_code(code).is_some() {
                        self.assign_selected(KeyAction::HidKey { code,
                            ctrl: self.key_picker_modifiers.0, shift: self.key_picker_modifiers.1,
                            alt: self.key_picker_modifiers.2, win: self.key_picker_modifiers.3 });
                    }
                }
            }
            ui.small("Key and modifier selections change the local draft. Save to Keyboard sends it.");
        }
        if ui.button("Disable key in draft").clicked() { self.assign_selected(KeyAction::Disabled); }
        ui.separator();
        ui.add_enabled(false, egui::Button::new("Macros / mouse / launch actions"));
        ui.small("Additional binding categories are not implemented yet.");
    }

    fn footer(&mut self, ui: &mut egui::Ui, rect: Rect) {
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            let connected = matches!(self.device.status(), DeviceStatus::Connected { .. });
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((rect.width() - 220.0).max(200.0));
                    if self.local_profile.is_some() {
                        let scope = match self.page {
                            Page::Keybinding => "keybindings only",
                            Page::Lighting => "base effect, supplied custom colors and input latency",
                            _ => "keys, lighting and performance",
                        };
                        ui.small(format!("Writes {scope}: profile {}, {}.", self.onboard_slot + 1,
                            if self.page == Page::Home { "all layers".into() } else { format!("layer {}", self.edit_layer + 1) }));
                        if self.page == Page::Lighting {
                            ui.small("Also activates this slot/layer; both lighting subpages are included.");
                        }
                    }
                    if self.dirty { ui.colored_label(ACCENT, "Unexported changes (local draft)"); }
                    ui.label(self.pending_write.as_ref().unwrap_or(&self.status_line));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if self.page == Page::Home { "Apply all layers in slot" } else { "Save to Keyboard" };
                    if ui.add_enabled(connected && self.local_profile.is_some() && self.pending_write.is_none(), egui::Button::new(label).min_size(egui::vec2(170.0, 42.0)))
                        .on_hover_text("Explicit hardware write. Export your draft first; persistence is not read back.").clicked() {
                        if self.page == Page::Home {
                            let layers = self.local_profile.as_ref().unwrap().profiles[self.onboard_slot as usize].layers.clone();
                            if let Ok(layers) = layers.try_into() {
                                let result = self.device.apply_full_profile(self.onboard_slot, layers);
                                self.record_write_submission(result, format!("Applying all layers in profile {}", self.onboard_slot + 1));
                            }
                        } else if let Some(layer) = self.current_layer().cloned() {
                            let result = if self.page == Page::Keybinding {
                                self.device.apply_keybindings(self.onboard_slot, self.edit_layer, layer.key_matrix)
                            } else if self.page == Page::Lighting {
                                self.device.apply_lighting(self.onboard_slot, self.edit_layer, layer.lighting)
                            } else {
                                self.device.apply_layer(self.onboard_slot, self.edit_layer, layer)
                            };
                            self.record_write_submission(result, format!("Saving {} for profile {}, layer {}", self.page.label(), self.onboard_slot + 1, self.edit_layer + 1));
                        }
                    }
                });
            });
        });
    }

    fn device_tools(&mut self, context: &egui::Context) {
        let mut open = self.tools_open;
        egui::Window::new("Device and local files").open(&mut open).default_width(430.0).show(context, |ui| {
            let connected = matches!(self.device.status(), DeviceStatus::Connected { .. });
            let idle = self.pending_write.is_none() && !self.worker_stopped;
            ui.horizontal(|ui| {
                if ui.add_enabled(!connected && idle, egui::Button::new("Connect")).clicked() { self.device.connect(); }
                if ui.add_enabled(connected && idle, egui::Button::new("Disconnect")).clicked() { self.device.disconnect(); }
                if ui.add_enabled(connected && idle, egui::Button::new("Refresh info"))
                    .on_hover_text(FIRMWARE_INFO_HELP).clicked() { self.device.refresh_info(); }
            });
            ui.label("Connection does not load or write onboard settings.");
            ui.small(&self.profile_notification_status);
            if let Some((profile, layer)) = self.profile_notification {
                ui.label(format!("Last keyboard notification: slot {}, layer {}", profile + 1, layer + 1));
                ui.small("An observed slot/layer change only. Local settings are unchanged; full onboard settings remain unknown.");
            }
            ui.add_enabled(false, egui::Button::new("Read onboard settings (unverified)"))
                .on_disabled_hover_text(ONBOARD_READ_UNAVAILABLE);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Import profile…").clicked() { self.choose_import(); }
                if ui.add_enabled(self.local_profile.is_some(), egui::Button::new("Export draft…")).clicked() { self.choose_export(); }
            });
            if ui.add_enabled(connected && self.local_profile.is_some() && idle, egui::Button::new("Activate selected slot / layer"))
                .on_hover_text("Changes the keyboard's active slot/layer explicitly; does not download settings.").clicked() {
                let result = self.device.switch_profile(self.onboard_slot, self.edit_layer);
                self.record_write_submission(result, format!("Activating profile {}, layer {}", self.onboard_slot + 1, self.edit_layer + 1));
            }
            ui.label(self.pending_write.as_ref().unwrap_or(&self.last_op));
        });
        self.tools_open = open;
    }

    pub(super) fn choose_import(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("GMMK profile", &["json"]).pick_file() {
            if self.dirty { self.pending_file = Some(path); } else { self.import_path(path); }
        }
    }

    pub(super) fn choose_export(&mut self) {
        if let Some(path) = rfd::FileDialog::new().set_file_name("my-gmmk.gmmkprofile.json").save_file() { self.export_to(path); }
    }

    fn import_confirmation(&mut self, context: &egui::Context) {
        if self.pending_file.is_none() { return; }
        egui::Window::new("Replace unexported draft?").collapsible(false).resizable(false).show(context, |ui| {
            ui.label("Import replaces the local draft. It does not change the keyboard.");
            ui.horizontal(|ui| {
                if ui.button("Keep current draft").clicked() { self.pending_file = None; }
                if ui.button("Replace draft").clicked() {
                    if let Some(path) = self.pending_file.take() { self.import_path(path); }
                }
            });
        });
    }
}
