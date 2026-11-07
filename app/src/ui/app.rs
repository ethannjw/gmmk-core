use crate::device::{DeviceEvent, DeviceHandle, DeviceStatus, FIRMWARE_INFO_HELP, ONBOARD_READ_UNAVAILABLE};
use crate::layout::KeyboardLayout;
use crate::profile::{
    apply_key_action, decode_key_action, parse_profile_json, serialize_profile,
    KeyAction, KeyboardCategory, LayerState, MediaAction, PerKeyColor, ProfileFile,
};
#[cfg(test)]
use crate::protocol::LED_EFFECT_IDS;
use eframe::egui;
use std::path::PathBuf;

mod view;
mod entry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Screen {
    #[default]
    Devices,
    Keyboard,
}

const LIGHTING_KEY_GROUPS: &[(&str, &[&str])] = &[
    ("WASD", &["W", "A", "S", "D"]),
    ("Arrows", &["Up", "Left", "Down", "Right"]),
    ("F Keys", &["f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12"]),
    ("Modifiers", &["Shift", "RShift", "Ctrl", "Left Win", "Alt", "RAlt", "RCtrl"]),
    ("Alphas", &["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"]),
    ("Numbers", &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Home,
    Performance,
    Lighting,
    Keybinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindingCategory {
    Keyboard,
    Multimedia,
}

impl Page {
    const ALL: [Self; 4] = [Self::Home, Self::Performance, Self::Lighting, Self::Keybinding];

    fn label(self) -> &'static str {
        match self {
            Self::Home => "My Keyboard",
            Self::Performance => "Performance",
            Self::Lighting => "Lighting",
            Self::Keybinding => "Keybinding",
        }
    }
}

pub struct GmmkApp {
    layout: KeyboardLayout,
    device: DeviceHandle,
    local_profile: Option<ProfileFile>,
    onboard_slot: u8,
    edit_layer: u8,
    dirty: bool,
    status_line: String,
    last_op: String,
    pending_write: Option<String>,
    worker_stopped: bool,
    selected_key: Option<usize>,
    binding_category: BindingCategory,
    keyboard_category: KeyboardCategory,
    key_picker_hid: u8,
    include_key_modifier: bool,
    key_picker_modifiers: (bool, bool, bool, bool),
    pending_file: Option<PathBuf>,
    page: Page,
    screen: Screen,
    rename_profile: Option<(usize, String)>,
    restore_profile: Option<usize>,
    custom_lighting: bool,
    paint_hex: String,
    paint_breathing: bool,
    erase_lighting: bool,
    effect_page: usize,
    tools_open: bool,
    firmware: Option<String>,
    profile_notifications_listening: bool,
    profile_notification: Option<(u8, u8)>,
    profile_notification_status: String,
}

impl GmmkApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        super::appearance::configure(&cc.egui_ctx);
        Self::with_device(DeviceHandle::spawn())
    }

    fn with_device(device: DeviceHandle) -> Self {
        let layout = KeyboardLayout::load_embedded().expect("embedded layout");
        Self {
            layout,
            device,
            local_profile: None,
            onboard_slot: 0,
            edit_layer: 0,
            dirty: false,
            status_line: "Looking for your keyboard…".into(),
            last_op: String::new(),
            pending_write: None,
            worker_stopped: false,
            selected_key: None,
            binding_category: BindingCategory::Keyboard,
            keyboard_category: KeyboardCategory::Alphanumeric,
            key_picker_hid: 4,
            include_key_modifier: false,
            key_picker_modifiers: (false, false, false, false),
            pending_file: None,
            page: Page::Home,
            screen: Screen::Devices,
            rename_profile: None,
            restore_profile: None,
            custom_lighting: false,
            paint_hex: "#FF0000".into(),
            paint_breathing: false,
            erase_lighting: false,
            effect_page: 0,
            tools_open: false,
            firmware: None,
            profile_notifications_listening: false,
            profile_notification: None,
            profile_notification_status: "Connect to listen for slot/layer notifications.".into(),
        }
    }

    fn current_layer_mut(&mut self) -> Option<&mut LayerState> {
        let p = self.local_profile.as_mut()?;
        let prof = p.profiles.get_mut(self.onboard_slot as usize)?;
        prof.layers.get_mut(self.edit_layer as usize)
    }

    fn current_layer(&self) -> Option<&LayerState> {
        let p = self.local_profile.as_ref()?;
        let prof = p.profiles.get(self.onboard_slot as usize)?;
        prof.layers.get(self.edit_layer as usize)
    }

    fn paint_color(&self) -> Option<[u8; 3]> {
        let text = self.paint_hex.trim();
        let hex = text.strip_prefix('#').unwrap_or(text);
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(hex, 16).ok()?;
        Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
    }

    fn paint_keys(&mut self, keys: &[&str]) {
        let indices = keys.iter().filter_map(|key| self.layout.led_index_for_matrix_key(key)).collect();
        self.paint_led_indices(indices);
    }

    fn paint_all(&mut self) {
        let indices = self.layout.device.matrix_key_buttons.iter()
            .filter_map(|key| self.layout.led_index_for_matrix_key(key))
            .chain(crate::protocol::SIDE_LED_INDICES).collect();
        self.paint_led_indices(indices);
    }

    fn paint_led_indices(&mut self, mut indices: Vec<usize>) {
        indices.sort_unstable();
        indices.dedup();
        if indices.is_empty() { return; }
        let erase = self.erase_lighting;
        let color = match self.paint_color() {
            Some(color) => color,
            None if erase => [0; 3],
            None => return,
        };
        let visible = if self.paint_breathing { 254 } else { 255 };
        if let Some(layer) = self.current_layer_mut() {
            if erase && layer.lighting.per_key_colors.is_none() { return; }
            let mut colors = layer.lighting.per_key_colors.clone().unwrap_or_default();
            for matrix_index in indices {
                if erase {
                    colors.retain(|entry| entry.matrix_index != matrix_index);
                } else {
                    let entry = PerKeyColor { matrix_index, r: color[0], g: color[1], b: color[2], visible };
                    if let Some(existing) = colors.iter_mut().find(|entry| entry.matrix_index == matrix_index) {
                        *existing = entry;
                    } else {
                        colors.push(entry);
                    }
                }
            }
            if layer.lighting.per_key_colors.as_ref() != Some(&colors) {
                layer.lighting.per_key_colors = Some(colors);
                self.dirty = true;
            }
        }
    }

    fn poll_device(&mut self) {
        if self.worker_stopped { return; }
        loop {
            let evt = match self.device.events.try_recv() {
                Ok(event) => event,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let message = if self.pending_write.take().is_some() {
                        "Device worker stopped; pending write outcome is unknown. No automatic retry. Restart the app and verify the keyboard before writing again."
                    } else {
                        "Device worker stopped. Restart the app to reconnect."
                    };
                    self.worker_stopped = true;
                    self.firmware = None;
                    self.clear_profile_notification();
                    self.device.set_status(DeviceStatus::Error(message.into()));
                    self.status_line = message.into();
                    self.last_op = message.into();
                    break;
                }
            };
            match evt {
                DeviceEvent::Status(st) => {
                    self.clear_profile_notification();
                    if !matches!(st, DeviceStatus::Connected { .. }) { self.firmware = None; }
                    self.device.set_status(st.clone());
                    self.status_line = match st {
                        DeviceStatus::Connected { product } => format!("Connected: {product}"),
                        DeviceStatus::Disconnected => "Disconnected".into(),
                        DeviceStatus::Error(e) => format!("Device error: {e}"),
                    };
                }
                DeviceEvent::Info { fw_version, message } => {
                    self.firmware = fw_version;
                    self.last_op = message;
                }
                DeviceEvent::OpResult { ok, message } => {
                    self.pending_write = None;
                    self.last_op = message.clone();
                    self.status_line = if ok {
                        format!("Success: {message}")
                    } else {
                        format!("Failed: {message}")
                    };
                }
                DeviceEvent::NotificationStatus { listening, message } => {
                    self.profile_notifications_listening = listening
                        && matches!(self.device.status(), DeviceStatus::Connected { .. });
                    self.profile_notification = None;
                    self.profile_notification_status = message;
                }
                DeviceEvent::ProfileNotification { profile, layer } => {
                    if self.profile_notifications_listening {
                        self.profile_notification = Some((profile, layer));
                        self.profile_notification_status = "Listening for slot/layer notifications.".into();
                    }
                }
            }
        }
    }

    fn clear_profile_notification(&mut self) {
        self.profile_notifications_listening = false;
        self.profile_notification = None;
        self.profile_notification_status = "Connect to listen for slot/layer notifications.".into();
    }

    fn record_write_submission(&mut self, result: Result<(), String>, description: String) {
        match result {
            Ok(()) => {
                self.profile_notification = None;
                let message = format!("{description}; awaiting transport result.");
                self.pending_write = Some(message.clone());
                self.status_line = message;
            }
            Err(error) => {
                self.pending_write = None;
                self.worker_stopped = true;
                self.firmware = None;
                self.clear_profile_notification();
                self.device.set_status(DeviceStatus::Error(error.clone()));
                self.last_op = error.clone();
                self.status_line = error;
            }
        }
    }

    fn import_path(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_profile_json(&text) {
                Ok(file) => {
                    self.onboard_slot = file.active_profile;
                    self.edit_layer = 0;
                    self.selected_key = None;
                    self.local_profile = Some(file);
                    self.screen = Screen::Keyboard;
                    self.rename_profile = None;
                    self.restore_profile = None;
                    self.sync_effect_page();
                    self.dirty = false;
                    self.status_line = format!("Imported {}", path.display());
                }
                Err(e) => self.status_line = format!("Import failed: {e}"),
            },
            Err(e) => self.status_line = format!("Read failed: {e}"),
        }
    }

    fn select_key(&mut self, index: usize) {
        if index >= self.layout.device.matrix_key_buttons.len() {
            return;
        }
        self.selected_key = Some(index);
        self.keyboard_category = KeyboardCategory::Alphanumeric;
        self.key_picker_hid = 0;
        self.include_key_modifier = false;
        self.key_picker_modifiers = (false, false, false, false);
        if let Some(layer) = self.current_layer() {
            match decode_key_action(&layer.key_matrix, &self.layout, index) {
                KeyAction::HidKey { code, ctrl, shift, alt, win } => {
                    self.binding_category = BindingCategory::Keyboard;
                    self.key_picker_hid = code;
                    self.keyboard_category = KeyboardCategory::for_code(code).unwrap_or(KeyboardCategory::Others);
                    self.key_picker_modifiers = (ctrl, shift, alt, win);
                    self.include_key_modifier = ctrl || shift || alt || win;
                }
                KeyAction::Multimedia(_) => self.binding_category = BindingCategory::Multimedia,
                _ => self.binding_category = BindingCategory::Keyboard,
            }
        }
    }

    fn select_layer(&mut self, layer: u8) {
        if layer < 3 {
            self.edit_layer = layer;
            self.sync_effect_page();
            if let Some(index) = self.selected_key { self.select_key(index); }
        }
    }

    fn select_slot(&mut self, slot: u8) {
        if slot < 3 {
            self.onboard_slot = slot;
            self.selected_key = None;
            self.sync_effect_page();
        }
    }

    fn sync_effect_page(&mut self) {
        self.effect_page = self.current_layer().map(|layer|
            super::preset_effects::initial_page(layer.lighting.effect_ui_value)).unwrap_or(0);
    }

    fn sync_paint_color(&mut self) {
        let rgb = self.current_layer().map(|layer| layer.lighting.rgb).unwrap_or([255, 0, 0]);
        self.paint_hex = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
    }

    fn select_lighting_mode(&mut self, custom: bool) {
        if self.custom_lighting != custom {
            self.custom_lighting = custom;
            if !custom {
                self.sync_paint_color();
                self.sync_effect_page();
            }
        }
    }

    fn select_page(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            if page == Page::Lighting {
                self.custom_lighting = false;
                self.paint_breathing = false;
                self.erase_lighting = false;
                self.sync_paint_color();
                self.sync_effect_page();
            }
        }
    }

    fn assign_selected(&mut self, action: KeyAction) {
        let Some(index) = self.selected_key else { return };
        let layout = self.layout.clone();
        if let Some(layer) = self.current_layer_mut() {
            let before = layer.key_matrix.clone();
            apply_key_action(&mut layer.key_matrix, &layout, index, action);
            self.dirty |= layer.key_matrix != before;
        }
    }

    fn export_to(&mut self, path: PathBuf) {
        let result = self.local_profile.as_ref()
            .ok_or_else(|| "no local profile to export".to_string())
            .and_then(serialize_profile)
            .and_then(|text| std::fs::write(&path, text).map_err(|error| error.to_string()));
        match result {
            Ok(()) => {
                self.dirty = false;
                self.status_line = format!("Exported local profile to {}", path.display());
            }
            Err(error) => self.status_line = format!("Export failed: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> (GmmkApp, std::sync::mpsc::Sender<DeviceEvent>) {
        let (device, events) = crate::device::tests::isolated_handle();
        let mut app = GmmkApp::with_device(device);
        app.local_profile = Some(parse_profile_json(include_str!(
            "../../assets/factory_profile.gmmkprofile.json"
        )).unwrap());
        app.dirty = true;
        app.screen = Screen::Keyboard;
        (app, events)
    }

    #[test]
    fn device_entry_starts_at_chooser_without_creating_or_writing_a_profile() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let context = egui::Context::default();
        let output = frame(&mut app, &context, vec![]);
        let labels = text_shapes(&output);
        assert!(labels.iter().any(|(text, _)| text == "Select your device"));
        assert!(labels.iter().any(|(text, _)| text == "No supported keyboard connected"));
        assert!(!labels.iter().any(|(text, _)| text == "My Keyboard"));
        assert!(app.local_profile.is_none());
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn device_entry_opens_local_defaults_and_preserves_draft_across_back_and_reconnect() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let context = egui::Context::default();
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "GMMK 2".into() })).unwrap();
        click_label(&mut app, &context, "GMMK V2 96 ANSI");
        let output = frame(&mut app, &context, vec![]);
        assert!(text_shapes(&output).iter().any(|(text, _)| text == "Local profile · onboard values unknown"));
        assert!(app.local_profile.is_some());
        app.local_profile.as_mut().unwrap().profiles[0].name = "My draft".into();
        app.dirty = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        click_label(&mut app, &context, "Devices");
        events.send(DeviceEvent::Status(DeviceStatus::Disconnected)).unwrap();
        frame(&mut app, &context, vec![]);
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "GMMK 2".into() })).unwrap();
        click_label(&mut app, &context, "GMMK V2 96 ANSI");
        click_label(&mut app, &context, "Profile 2");
        assert_eq!(app.onboard_slot, 1);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn device_entry_permission_error_never_creates_a_connected_card() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        events.send(DeviceEvent::Status(DeviceStatus::Error("open failed: not permitted".into()))).unwrap();
        let context = egui::Context::default();
        let output = frame(&mut app, &context, vec![]);
        let labels = text_shapes(&output);
        assert!(labels.iter().any(|(text, _)| text == "Keyboard access unavailable"));
        assert!(!labels.iter().any(|(text, _)| text == "GMMK V2 96 ANSI"));
        assert!(app.local_profile.is_none());
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn chooser_displays_file_errors_without_discarding_the_draft() {
        let (mut app, _events) = app();
        app.screen = Screen::Devices;
        let context = egui::Context::default();
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        app.import_path(directory.clone());
        let output = frame(&mut app, &context, vec![]);
        assert!(text_shapes(&output).iter().any(|(text, _)| text.starts_with("Read failed:")));
        app.export_to(directory);
        let output = frame(&mut app, &context, vec![]);
        assert!(text_shapes(&output).iter().any(|(text, _)| text.starts_with("Export failed:")));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(app.dirty);
    }

    #[test]
    fn profile_menu_rename_and_confirmed_reset_are_local_and_slot_scoped() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "...");
        let output = frame(&mut app, &context, vec![]);
        for expected in ["Rename", "Import profile…", "Export draft…", "Restore local defaults"] {
            assert!(text_shapes(&output).iter().any(|(text, _)| text == expected), "missing {expected}");
        }
        click_label(&mut app, &context, "Rename");
        app.rename_profile.as_mut().unwrap().1 = "Work".into();
        click_label(&mut app, &context, "Save name");
        assert_eq!(app.local_profile.as_ref().unwrap().profiles[0].name, "Work");
        assert!(app.dirty);
        app.local_profile.as_mut().unwrap().profiles[1].name = "Keep me".into();
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        click_label(&mut app, &context, "...");
        click_label(&mut app, &context, "Restore local defaults");
        frame(&mut app, &context, vec![]);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        click_label(&mut app, &context, "Keep profile");
        assert!(app.restore_profile.is_none());
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        click_label(&mut app, &context, "...");
        click_label(&mut app, &context, "Restore local defaults");
        click_label(&mut app, &context, "Restore this local profile");
        assert_eq!(app.local_profile.as_ref().unwrap().profiles[0].name, "Profile 1");
        assert_eq!(app.local_profile.as_ref().unwrap().profiles[1].name, "Keep me");
        assert!(app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn keyboard_notifications_display_observations_without_changing_local_settings_or_selection() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.select_slot(2);
        app.select_layer(1);
        app.dirty = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        events.send(DeviceEvent::NotificationStatus { listening: true, message: "Listening".into() }).unwrap();
        events.send(DeviceEvent::ProfileNotification { profile: 0, layer: 2 }).unwrap();
        app.tools_open = true;
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        let output = frame(&mut app, &context, vec![]);
        assert!(text_shapes(&output).iter().any(|(text, _)| text == "Last keyboard notification: slot 1, layer 3"));
        assert_eq!(app.profile_notification, Some((0, 2)));
        assert_eq!((app.onboard_slot, app.edit_layer), (2, 1));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn keyboard_notifications_do_not_create_a_local_profile_or_complete_a_pending_write() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        events.send(DeviceEvent::NotificationStatus { listening: true, message: "Listening".into() }).unwrap();
        app.poll_device();
        app.profile_notification = Some((0, 0));
        app.record_write_submission(Ok(()), "Test pending write".into());
        assert_eq!(app.profile_notification, None);
        let pending = app.pending_write.clone();
        events.send(DeviceEvent::ProfileNotification { profile: 2, layer: 1 }).unwrap();
        app.poll_device();
        assert!(app.local_profile.is_none());
        assert!(!app.dirty);
        assert_eq!(app.profile_notification, Some((2, 1)));
        assert_eq!(app.pending_write, pending);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn keyboard_notifications_clear_on_reader_failure_and_disconnect_and_ignore_late_events() {
        let (mut app, events) = app();
        for stop_reader in [true, false] {
            events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
            events.send(DeviceEvent::NotificationStatus { listening: true, message: "Listening".into() }).unwrap();
            events.send(DeviceEvent::ProfileNotification { profile: 1, layer: 2 }).unwrap();
            app.poll_device();
            assert_eq!(app.profile_notification, Some((1, 2)));
            if stop_reader {
                events.send(DeviceEvent::NotificationStatus { listening: false, message: "Reader stopped".into() }).unwrap();
            } else {
                events.send(DeviceEvent::Status(DeviceStatus::Disconnected)).unwrap();
            }
            events.send(DeviceEvent::ProfileNotification { profile: 0, layer: 0 }).unwrap();
            app.poll_device();
            assert_eq!(app.profile_notification, None);
            assert!(!app.profile_notifications_listening);
            if stop_reader {
                assert_eq!(app.profile_notification_status, "Reader stopped");
                assert!(matches!(app.device.status(), DeviceStatus::Connected { .. }));
            }
        }
    }

    #[test]
    fn keyboard_notifications_clear_when_the_device_worker_stops() {
        let (mut app, events) = app();
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        events.send(DeviceEvent::NotificationStatus { listening: true, message: "Listening".into() }).unwrap();
        events.send(DeviceEvent::ProfileNotification { profile: 1, layer: 2 }).unwrap();
        app.poll_device();
        assert_eq!(app.profile_notification, Some((1, 2)));
        drop(events);
        app.poll_device();
        assert_eq!(app.profile_notification, None);
        assert!(!app.profile_notifications_listening);
        assert!(app.worker_stopped);
    }

    #[test]
    fn hardware_results_do_not_discard_unexported_local_changes() {
        let (mut app, events) = app();
        for ok in [true, false] {
            events.send(DeviceEvent::OpResult {
                ok,
                message: "slot operation".into(),
            }).unwrap();
            app.poll_device();
            assert!(app.dirty);
        }
    }

    #[test]
    fn failed_export_remains_dirty_and_displays_the_error() {
        let (mut app, _events) = app();
        app.export_to(std::env::temp_dir());
        assert!(app.dirty);
        assert!(app.status_line.starts_with("Export failed:"), "{}", app.status_line);
    }

    #[test]
    fn successful_export_preserves_the_draft_and_clears_only_local_dirty_state() {
        let (mut app, _events) = app();
        let path = std::env::temp_dir().join(format!(
            "gmmk-export-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let matrix = &mut app.local_profile.as_mut().unwrap().profiles[0].layers[0].key_matrix;
        matrix[0] = 251;
        matrix[1] = 193;
        app.export_to(path.clone());
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let restored = parse_profile_json(&text).unwrap();
        assert_eq!(&restored.profiles[0].layers[0].key_matrix[..2], &[251, 193]);
        assert!(!app.dirty);
        assert!(app.status_line.starts_with("Exported local profile"), "{}", app.status_line);
        assert!(matches!(app.device.status(), DeviceStatus::Disconnected));
    }

    fn frame(app: &mut GmmkApp, context: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        crate::ui::appearance::configure(context);
        context.run(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 880.0))),
            events,
            ..Default::default()
        }, |context| app.show(context))
    }

    #[test]
    fn lighting_write_failure_stays_inside_the_window_with_a_dirty_draft() {
        let (mut app, _events) = app();
        app.page = Page::Lighting;
        app.dirty = true;
        app.status_line = "Failed: Report 1/8 failed after 0 completed transfers; device state may be partially changed: SetFeatureReport failed: hidapi error: IOHIDDeviceSetReport failed: (0xE0005000) unknown error code. No automatic retry.".into();
        for size in [egui::vec2(1440.0, 880.0), egui::vec2(1000.0, 700.0)] {
            let context = egui::Context::default();
            crate::ui::appearance::configure(&context);
            let output = context.run(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            }, |context| app.show(context));
            let error = text_shapes(&output).into_iter().find(|(text, _)| text == &app.status_line).unwrap();
            assert!(error.1.bottom() <= size.y, "error extends below window: {:?}", error.1);
        }
    }

    fn frame_at(app: &mut GmmkApp, context: &egui::Context, time: f64, events: Vec<egui::Event>) -> egui::FullOutput {
        crate::ui::appearance::configure(context);
        context.run(egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 880.0))),
            events,
            ..Default::default()
        }, |context| app.show(context))
    }

    fn text_shapes(output: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        fn collect(shape: &egui::Shape, clip_rect: egui::Rect, labels: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => {
                    let rect = text.visual_bounding_rect();
                    if clip_rect.contains(rect.center()) { labels.push((text.galley.text().to_string(), rect)); }
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes { collect(shape, clip_rect, labels); }
                }
                _ => {}
            }
        }
        let mut labels = Vec::new();
        for shape in &output.shapes { collect(&shape.shape, shape.clip_rect, &mut labels); }
        labels
    }

    fn binding_tile_shape(output: &egui::FullOutput, label: &str) -> egui::epaint::RectShape {
        fn collect(shape: &egui::Shape, rectangles: &mut Vec<egui::epaint::RectShape>) {
            match shape {
                egui::Shape::Rect(rectangle) => rectangles.push(rectangle.clone()),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes { collect(shape, rectangles); }
                }
                _ => {}
            }
        }
        let position = text_shapes(output).into_iter().find(|(text, _)| text == label).unwrap().1.center();
        let mut rectangles = Vec::new();
        for shape in &output.shapes { collect(&shape.shape, &mut rectangles); }
        rectangles.into_iter().filter(|shape| shape.rect.contains(position))
            .min_by(|left, right| left.rect.area().total_cmp(&right.rect.area())).unwrap()
    }

    fn binding_text_color(output: &egui::FullOutput, label: &str) -> egui::Color32 {
        fn find(shape: &egui::Shape, label: &str) -> Option<egui::Color32> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.override_text_color.unwrap_or(text.galley.job.sections[0].format.color))
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        output.shapes.iter().find_map(|shape| find(&shape.shape, label)).unwrap()
    }

    #[test]
    fn media_tiles_render_reference_geometry_colors_and_hover_states() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        click_label(&mut app, &context, "Multimedia");
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let output = frame(&mut app, &context, vec![egui::Event::PointerGone]);
        let tile = binding_tile_shape(&output, "Mute");
        assert_eq!(tile.rect.size(), egui::vec2(117.0, 48.0));
        assert_eq!(tile.corner_radius, egui::CornerRadius::same(2));
        assert_eq!(tile.fill, egui::Color32::from_rgb(38, 38, 38));
        assert_eq!(tile.stroke, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(81, 81, 81)));
        assert_eq!(binding_text_color(&output, "Mute"), egui::Color32::WHITE);
        let output = frame(&mut app, &context, vec![egui::Event::PointerMoved(tile.rect.center())]);
        assert_eq!(binding_tile_shape(&output, "Mute").stroke.color, egui::Color32::from_rgb(112, 112, 112));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        let output = click_label(&mut app, &context, "Mute");
        let selected = binding_tile_shape(&output, "Mute");
        assert_eq!(selected.fill, egui::Color32::from_rgb(197, 180, 227));
        assert_eq!(selected.stroke.color, egui::Color32::from_rgb(132, 119, 190));
        assert_eq!(binding_text_color(&output, "Mute"), egui::Color32::from_rgb(28, 28, 28));
        let output = click_label(&mut app, &context, "Keybinding");
        assert_eq!(binding_tile_shape(&output, "Mute").stroke.color, egui::Color32::from_rgb(217, 206, 244));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn modifier_tiles_use_reference_geometry_and_wrapping_without_device_writes() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        click_label(&mut app, &context, "Include Modifier");
        let mut expected = app.local_profile.clone().unwrap();
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        expected.profiles[0].layers[0].key_matrix[led] = 225;
        click_label(&mut app, &context, "Ctrl");
        let output = frame(&mut app, &context, vec![egui::Event::PointerGone]);
        let selected = binding_tile_shape(&output, "Ctrl");
        assert_eq!(selected.rect.size(), egui::vec2(117.0, 48.0));
        assert_eq!(selected.corner_radius, egui::CornerRadius::same(2));
        assert_eq!(selected.fill, egui::Color32::from_rgb(197, 180, 227));
        let shift = binding_tile_shape(&output, "Shift");
        assert_eq!(shift.fill, egui::Color32::from_rgb(38, 38, 38));
        assert_eq!(selected.rect.left() - shift.rect.right(), 8.0);
        let meta = binding_tile_shape(&output, "Meta");
        assert_eq!(meta.rect.top() - selected.rect.bottom(), 8.0);
        assert!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap() == serialize_profile(&expected).unwrap());
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn chooser_renders_real_device_only_at_supported_window_sizes() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let context = egui::Context::default();
        crate::ui::appearance::configure(&context);
        for connected in [false, true] {
            if connected {
                events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
            }
            for size in [egui::vec2(1000.0, 700.0), egui::vec2(1440.0, 880.0)] {
                for page in Page::ALL {
                    app.page = page;
                    let output = context.run(egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        ..Default::default()
                    }, |context| app.show(context));
                    let labels = text_shapes(&output);
                    for expected in ["Select your device", "Import profile…"] {
                        assert!(labels.iter().any(|(text, _)| text == expected),
                            "missing {expected} on {page:?}, connected={connected}, size={size:?}");
                    }
                    assert_eq!(labels.iter().any(|(text, _)| text == "GMMK V2 96 ANSI"), connected);
                    assert!(!labels.iter().any(|(text, _)| text == "My Keyboard"));
                    assert!(app.local_profile.is_none());
                    assert!(!app.dirty);
                    assert!(app.selected_key.is_none());
                    assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
                }
            }
        }
    }

    #[test]
    fn keyboard_preview_is_read_only_until_an_explicit_local_profile_import() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.screen = Screen::Keyboard;
        let context = egui::Context::default();
        for page in [Page::Keybinding, Page::Lighting] {
            app.page = page;
            app.custom_lighting = page == Page::Lighting;
            click_label(&mut app, &context, "A");
            assert!(app.local_profile.is_none());
            assert!(app.selected_key.is_none());
            assert!(!app.dirty);
        }
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Keybinding;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let output = click_label(&mut app, &context, "A");
        assert!(app.selected_key.is_some());
        assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Keyboard layout preview — onboard settings unknown."));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn native_home_renders_reference_tabs_and_three_profile_tiles() {
        let (mut app, _events) = app();
        let context = egui::Context::default();
        let output = frame(&mut app, &context, vec![]);
        let labels = text_shapes(&output);
        for expected in ["My Keyboard", "Performance", "Lighting", "Keybinding", "Keyboard Profiles", "Profile 1", "Profile 2", "Profile 3"] {
            assert!(labels.iter().any(|(text, _)| text == expected), "missing {expected}: {labels:?}");
        }
    }

    #[test]
    fn device_header_uses_reference_text_sizes_and_selected_tab_color_without_writes() {
        fn text_format(shape: &egui::Shape, label: &str) -> Option<egui::TextFormat> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    let mut format = text.galley.job.sections[0].format.clone();
                    format.color = text.override_text_color.unwrap_or(format.color);
                    Some(format)
                }
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| text_format(shape, label)),
                _ => None,
            }
        }
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        let selected_color = egui::Color32::from_rgb(255, 164, 13);
        for page in Page::ALL {
            click_label(&mut app, &context, page.label());
            let output = frame(&mut app, &context, vec![]);
            let heading = output.shapes.iter().find_map(|shape| text_format(&shape.shape, "GMMK V2 96 ANSI")).unwrap();
            assert_eq!(heading.font_id.size, 20.0);
            assert_eq!(heading.color, egui::Color32::WHITE);
            for tab in Page::ALL {
                let label = output.shapes.iter().find_map(|shape| text_format(&shape.shape, tab.label())).unwrap();
                assert_eq!(label.font_id.size, 12.0, "{} tab size", tab.label());
                assert_eq!(label.color, if page == tab { selected_color } else { crate::ui::appearance::TEXT });
            }
            let underline = output.shapes.iter().filter(|shape| match &shape.shape {
                egui::Shape::LineSegment { stroke, .. } => stroke.color == selected_color && stroke.width == 2.0,
                _ => false,
            }).count();
            assert_eq!(underline, 1, "one selected-tab underline on {}", page.label());
            let other_page = if page == Page::Home { Page::Lighting } else { Page::Home };
            let other_position = text_shapes(&output).into_iter().find(|(text, _)| text == other_page.label()).unwrap().1.center();
            frame(&mut app, &context, vec![egui::Event::PointerMoved(other_position)]);
            assert_eq!(app.page, page);
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(!app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    fn click_label(app: &mut GmmkApp, context: &egui::Context, label: &str) -> egui::FullOutput {
        frame(app, context, vec![]);
        let mut output = frame(app, context, vec![]);
        for scroll in [0.0, 1500.0, -180.0, -180.0, -180.0, -180.0, -180.0, -180.0] {
            if scroll != 0.0 { output = scroll_controls(app, context, scroll); }
            if let Some((_, rect)) = text_shapes(&output).iter().find(|(text, _)| text == label) {
                return click_position(app, context, rect.center());
            }
        }
        panic!("missing clickable text after scrolling the controls: {label}")
    }

    fn scroll_controls(app: &mut GmmkApp, context: &egui::Context, amount: f32) -> egui::FullOutput {
        frame(app, context, vec![egui::Event::PointerMoved(egui::pos2(150.0, 500.0)), egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, amount), modifiers: Default::default(),
        }]);
        for _ in 0..30 { frame(app, context, vec![]); }
        frame(app, context, vec![])
    }

    fn lighting_control_labels(app: &mut GmmkApp, context: &egui::Context) -> Vec<&'static str> {
        let mut found = Vec::new();
        for scroll in [1500.0, -160.0, -160.0, -160.0, -160.0, -160.0] {
            let output = scroll_controls(app, context, scroll);
            for (text, _) in text_shapes(&output) {
                if let Some(label) = ["Effect Color", "Brightness", "Speed"].into_iter().find(|label| *label == text) {
                    if !found.contains(&label) { found.push(label); }
                }
            }
        }
        scroll_controls(app, context, 1500.0);
        found
    }

    fn click_popup_label(app: &mut GmmkApp, context: &egui::Context, label: &str) -> egui::FullOutput {
        frame(app, context, vec![]);
        let output = frame(app, context, vec![]);
        let position = text_shapes(&output).iter().rev().find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("missing popup text: {label}")).1.center();
        click_position(app, context, position)
    }

    fn click_position(app: &mut GmmkApp, context: &egui::Context, position: egui::Pos2) -> egui::FullOutput {
        frame(app, context, vec![egui::Event::PointerMoved(position), egui::Event::PointerButton {
            pos: position, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default(),
        }]);
        frame(app, context, vec![egui::Event::PointerButton {
            pos: position, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default(),
        }]);
        frame(app, context, vec![])
    }

    fn scroll_popup(app: &mut GmmkApp, context: &egui::Context, visible_label: &str) {
        let output = frame(app, context, vec![]);
        let position = text_shapes(&output).iter().rev().find(|(text, _)| text == visible_label).unwrap().1.center();
        frame(app, context, vec![egui::Event::PointerMoved(position), egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -1000.0), modifiers: Default::default(),
        }]);
        for _ in 0..30 { frame(app, context, vec![]); }
    }

    #[test]
    fn tab_navigation_and_key_selection_do_not_mutate_drafts_or_send_device_commands() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        assert!(app.local_profile.is_none());
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        for (tab, heading) in [("Performance", "General Performance"), ("Lighting", "Base Effect"), ("Keybinding", "Key Binding")] {
            let output = click_label(&mut app, &context, tab);
            assert!(text_shapes(&output).iter().any(|(text, _)| text == heading), "{tab} opens {heading}");
        }
        click_label(&mut app, &context, "A");
        assert_eq!(app.layout.device.matrix_key_buttons[app.selected_key.unwrap()], "A");
        assert_eq!(app.key_picker_hid, 4);
        click_label(&mut app, &context, "Layer 2");
        assert_eq!(app.edit_layer, 1);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn selecting_a_key_and_layer_loads_that_binding_instead_of_stale_picker_values() {
        let (mut app, _events) = app();
        let index = app.layout.device.matrix_key_buttons.iter().position(|key| key == "A").unwrap();
        let layout = app.layout.clone();
        let target = &mut app.local_profile.as_mut().unwrap().profiles[0].layers[1];
        apply_key_action(&mut target.key_matrix, &layout, index, KeyAction::HidKey {
            code: 5, ctrl: true, shift: false, alt: true, win: false,
        });
        app.select_key(index);
        assert_eq!(app.key_picker_hid, 4);
        app.select_layer(1);
        assert_eq!(app.key_picker_hid, 5);
        assert_eq!(app.key_picker_modifiers, (true, false, true, false));
        app.select_slot(1);
        assert_eq!(app.selected_key, None);
    }

    #[test]
    fn keyboard_category_browsing_preserves_the_draft_and_key_selection_assigns_locally() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let before = app.current_layer().unwrap().key_matrix.clone();
        click_label(&mut app, &context, "Alphanumeric");
        click_label(&mut app, &context, "Modifiers");
        assert_eq!(app.current_layer().unwrap().key_matrix, before);
        click_label(&mut app, &context, "Select key");
        click_label(&mut app, &context, "ShiftLeft");
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        let matrix = &app.current_layer().unwrap().key_matrix;
        assert_eq!((matrix[led], matrix[crate::protocol::KEY_MATRIX_LEN + led]), (1, 225));
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn reference_modifier_tiles_update_the_draft_immediately_and_wait_for_explicit_save() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        let initial = app.current_layer().unwrap().key_matrix.clone();
        let output = click_label(&mut app, &context, "Include Modifier");
        let labels = text_shapes(&output);
        assert_eq!(app.current_layer().unwrap().key_matrix, initial);
        assert!(!app.dirty);
        for name in ["Shift", "Ctrl", "Alt", "Meta"] {
            assert!(labels.iter().any(|(text, _)| text == name));
        }
        for (name, expected_type) in [("Shift", 226), ("Ctrl", 225), ("Alt", 228), ("Meta", 232)] {
            click_label(&mut app, &context, name);
            let mut expected = initial.clone();
            expected[led] = expected_type;
            assert_eq!(app.current_layer().unwrap().key_matrix, expected);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
        click_label(&mut app, &context, "Alphanumeric");
        click_popup_label(&mut app, &context, "F-Row");
        click_label(&mut app, &context, "Ctrl");
        let mut expected = initial.clone();
        expected[led] = 225;
        assert_eq!(app.current_layer().unwrap().key_matrix, expected);
        let output = click_label(&mut app, &context, "Include Modifier");
        assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Meta"));
        assert_eq!(app.current_layer().unwrap().key_matrix, initial);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        click_label(&mut app, &context, "Save to Keyboard");
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyKeybindings { profile: 0, layer: 0, key_matrix } => assert_eq!(key_matrix, initial),
            command => panic!("unexpected command: {command:?}"),
        }
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn imported_combined_modifiers_survive_until_the_user_explicitly_replaces_them() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        app.local_profile.as_mut().unwrap().profiles[0].layers[1].key_matrix[led] = 239;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let output = click_label(&mut app, &context, "Layer 2");
        assert!(text_shapes(&output).iter().any(|(text, _)| text.contains("Imported combined modifiers")));
        assert_eq!(app.key_picker_modifiers, (true, true, true, true));
        assert_eq!(app.current_layer().unwrap().key_matrix[led], 239);
        assert!(!app.dirty);
        let exported = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        assert_eq!(parse_profile_json(&exported).unwrap().profiles[0].layers[1].key_matrix[led], 239);
        click_label(&mut app, &context, "Ctrl");
        assert_eq!(app.current_layer().unwrap().key_matrix[led], 225);
        assert!(app.dirty);
        let output = click_label(&mut app, &context, "Layer 1");
        assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Meta"));
        assert_eq!(app.key_picker_modifiers, (false, false, false, false));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn selecting_a_keyboard_choice_replaces_added_modifiers_even_when_reselecting_the_same_key() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let mut expected = app.local_profile.clone().unwrap();
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        click_label(&mut app, &context, "Include Modifier");
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        expected.profiles[0].layers[0].key_matrix[crate::protocol::KEY_MATRIX_LEN + led] = 5;
        for previous in ["A", "B"] {
            click_label(&mut app, &context, "Ctrl");
            assert_eq!(app.current_layer().unwrap().key_matrix[led], 225);
            click_label(&mut app, &context, previous);
            let output = click_popup_label(&mut app, &context, "B");
            assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Assign to draft"));
            assert_eq!(app.key_picker_modifiers, (false, false, false, false));
            assert!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap() == serialize_profile(&expected).unwrap());
            assert!(app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn unsupported_disabled_and_media_bindings_do_not_reuse_previous_picker_values() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let index = app.selected_key.unwrap();
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        let cases = [(1, 0), (1, 251), (3, 255), (251, 193)].into_iter()
            .chain(MediaAction::ALL.into_iter().map(|media| (3, media as u8)));
        for (binding_type, code) in cases {
            let matrix = &mut app.current_layer_mut().unwrap().key_matrix;
            matrix[led] = binding_type;
            matrix[crate::protocol::KEY_MATRIX_LEN + led] = code;
            app.key_picker_hid = 4;
            app.key_picker_modifiers = (true, true, true, true);
            app.select_key(index);
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            click_label(&mut app, &context, "Keyboard Function");
            let output = frame(&mut app, &context, vec![]);
            assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Assign to draft"));
            click_label(&mut app, &context, "Include Modifier");
            click_label(&mut app, &context, "Ctrl");
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(!app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn keyboard_picker_loads_all_verified_imported_codes_and_modifiers_without_mutation() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let index = app.layout.device.matrix_key_buttons.iter().position(|key| key == "A").unwrap();
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        for category in KeyboardCategory::ALL {
            for choice in category.choices().iter().filter(|choice| choice.code.is_some()) {
                app.select_layer(0);
                app.binding_category = BindingCategory::Multimedia;
                let matrix = &mut app.local_profile.as_mut().unwrap().profiles[0].layers[1].key_matrix;
                matrix[led] = 239;
                matrix[crate::protocol::KEY_MATRIX_LEN + led] = choice.code.unwrap();
                let before = matrix.clone();
                app.select_key(index);
                app.select_layer(1);
                assert_eq!(app.binding_category, BindingCategory::Keyboard);
                assert_eq!(app.keyboard_category, category);
                assert_eq!(app.key_picker_hid, choice.code.unwrap());
                assert_eq!(app.key_picker_modifiers, (true, true, true, true));
                assert_eq!(app.current_layer().unwrap().key_matrix, before);
                assert!(!app.dirty);
            }
        }
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn keyboard_function_punctuation_and_navigation_choices_edit_only_the_selected_draft() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 2;
        let original = app.local_profile.clone().unwrap();
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let index = app.selected_key.unwrap();
        for (category, label, code) in [("F-Row", "F1", 58), ("Special Characters", "/", 56), ("Others", "▲", 82)] {
            let previous_category = app.keyboard_category.label();
            click_label(&mut app, &context, previous_category);
            if category == "Others" { scroll_popup(&mut app, &context, "Alphanumeric"); }
            click_popup_label(&mut app, &context, category);
            let before = app.current_layer().unwrap().key_matrix.clone();
            click_label(&mut app, &context, "Select key");
            if category == "Special Characters" { scroll_popup(&mut app, &context, "`"); }
            click_popup_label(&mut app, &context, label);
            assert_ne!(app.current_layer().unwrap().key_matrix, before);
            assert_eq!(decode_key_action(&app.current_layer().unwrap().key_matrix, &app.layout, index),
                KeyAction::HidKey { code, ctrl: false, shift: false, alt: false, win: false });
            let mut restored = parse_profile_json(&serialize_profile(app.local_profile.as_ref().unwrap()).unwrap()).unwrap();
            restored.profiles[2].layers[2].key_matrix = original.profiles[2].layers[2].key_matrix.clone();
            assert!(serialize_profile(&restored).unwrap() == serialize_profile(&original).unwrap());
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn unresolved_keyboard_choice_cannot_be_selected_or_assigned() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let before = app.current_layer().unwrap().key_matrix.clone();
        click_label(&mut app, &context, "Alphanumeric");
        click_popup_label(&mut app, &context, "Special Characters");
        click_label(&mut app, &context, "Select key");
        scroll_popup(&mut app, &context, "`");
        click_popup_label(&mut app, &context, "#");
        assert_eq!(app.key_picker_hid, 4);
        frame(&mut app, &context, vec![egui::Event::Key {
            key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default(),
        }]);
        assert_eq!(app.current_layer().unwrap().key_matrix, before);
        assert!(!app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn every_media_tile_targets_the_selected_layer_and_round_trips_without_sending() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        let before = app.local_profile.clone().unwrap();
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        click_label(&mut app, &context, "Multimedia");
        let index = app.selected_key.unwrap();
        for action in MediaAction::ALL {
            click_label(&mut app, &context, action.label());
            assert_eq!(decode_key_action(&app.current_layer().unwrap().key_matrix, &app.layout, index), KeyAction::Multimedia(action));
            let exported = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            let mut restored = parse_profile_json(&exported).unwrap();
            assert_eq!(serialize_profile(&restored).unwrap(), exported);
            restored.profiles[2].layers[1].key_matrix = before.profiles[2].layers[1].key_matrix.clone();
            assert_eq!(serialize_profile(&restored).unwrap(), serialize_profile(&before).unwrap());
            assert!(app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn key_and_layer_selection_load_media_category_without_mutation() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let index = app.layout.device.matrix_key_buttons.iter().position(|key| key == "A").unwrap();
        let layout = app.layout.clone();
        apply_key_action(&mut app.local_profile.as_mut().unwrap().profiles[0].layers[1].key_matrix, &layout, index, KeyAction::Multimedia(MediaAction::OpenMediaPlayer));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        app.dirty = false;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        assert_eq!(app.binding_category, BindingCategory::Keyboard);
        let output = click_label(&mut app, &context, "Layer 2");
        assert_eq!(app.binding_category, BindingCategory::Multimedia);
        assert!(text_shapes(&output).iter().any(|(text, _)| text == "Open Media Player"));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        click_label(&mut app, &context, "Disable key in draft");
        assert_eq!(decode_key_action(&app.current_layer().unwrap().key_matrix, &app.layout, index), KeyAction::Disabled);
        click_label(&mut app, &context, "Layer 1");
        assert_eq!(app.binding_category, BindingCategory::Keyboard);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn selecting_a_media_action_edits_only_the_draft_until_save() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let before = app.current_layer().unwrap().key_matrix.clone();
        click_label(&mut app, &context, "Multimedia");
        assert_eq!(app.current_layer().unwrap().key_matrix, before);
        click_label(&mut app, &context, "Mute");
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        let matrix = &app.current_layer().unwrap().key_matrix;
        assert_eq!((matrix[led], matrix[crate::protocol::KEY_MATRIX_LEN + led]), (3, 2));
        for offset in 0..matrix.len() {
            if offset != led && offset != crate::protocol::KEY_MATRIX_LEN + led {
                assert_eq!(matrix[offset], before[offset]);
            }
        }
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        let expected = matrix.clone();
        click_label(&mut app, &context, "Save to Keyboard");
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyKeybindings { profile: 0, layer: 0, key_matrix } => assert_eq!(key_matrix, expected),
            other => panic!("unexpected save: {other:?}"),
        }
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn assigning_a_binding_changes_only_the_selected_key_in_the_local_draft() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "A");
        let before = app.current_layer().unwrap().key_matrix.clone();
        click_label(&mut app, &context, "Disable key in draft");
        let after = &app.current_layer().unwrap().key_matrix;
        let led = app.layout.led_index_for_matrix_key("A").unwrap();
        for index in 0..after.len() {
            if index != crate::protocol::KEY_MATRIX_LEN + led { assert_eq!(after[index], before[index]); }
        }
        assert_eq!(after[crate::protocol::KEY_MATRIX_LEN + led], 0);
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn save_is_disabled_when_disconnected_and_only_explicit_save_queues_a_write() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        frame(&mut app, &context, vec![]);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv().unwrap(), crate::device::DeviceCommand::ApplyKeybindings { profile: 0, layer: 0, .. }));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn every_save_path_blocks_repeat_writes_until_success_or_failure_returns() {
        for page in Page::ALL {
            for ok in [true, false] {
                let (device, events, commands) = crate::device::tests::observed_handle();
                device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
                let mut app = GmmkApp::with_device(device);
                app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
                let context = egui::Context::default();
                click_label(&mut app, &context, page.label());
                let label = if page == Page::Home { "Apply all layers in slot" } else { "Save to Keyboard" };
                click_label(&mut app, &context, label);
                let submitted = commands.try_recv().unwrap();
                events.send(DeviceEvent::Info { fw_version: None, message: "cached metadata".into() }).unwrap();
                events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
                click_label(&mut app, &context, label);
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)), "repeat write on {page:?}");
                click_label(&mut app, &context, "My Keyboard");
                click_label(&mut app, &context, "Profile 2");
                click_label(&mut app, &context, "Keybinding");
                click_label(&mut app, &context, "A");
                click_label(&mut app, &context, "Disable key in draft");
                let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
                let output = click_label(&mut app, &context, "Save to Keyboard");
                assert!(text_shapes(&output).iter().any(|(text, _)| text.contains("profile 1") && text.contains("awaiting transport result")));
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
                match submitted {
                    crate::device::DeviceCommand::ApplyFullProfile { profile: 0, .. }
                    | crate::device::DeviceCommand::ApplyLayer { profile: 0, layer: 0, .. }
                    | crate::device::DeviceCommand::ApplyLighting { profile: 0, layer: 0, .. }
                    | crate::device::DeviceCommand::ApplyKeybindings { profile: 0, layer: 0, .. } => {}
                    command => panic!("unexpected write: {command:?}"),
                }
                events.send(DeviceEvent::OpResult { ok, message: "transport result".into() }).unwrap();
                frame(&mut app, &context, vec![]);
                assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
                assert!(app.dirty);
                assert!(app.status_line.starts_with(if ok { "Success:" } else { "Failed:" }));
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
                click_label(&mut app, &context, "Save to Keyboard");
                assert!(matches!(commands.try_recv(), Ok(crate::device::DeviceCommand::ApplyKeybindings { profile: 1, layer: 0, .. })));
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
            }
        }
    }

    #[test]
    fn pending_activation_blocks_other_device_commands_until_completion() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        app.tools_open = true;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Activate selected slot / layer");
        assert!(matches!(commands.try_recv(), Ok(crate::device::DeviceCommand::SwitchProfile { profile: 2, layer: 1 })));
        for label in ["Activate selected slot / layer", "Disconnect", "Connect", "Refresh info"] {
            click_label(&mut app, &context, label);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)), "queued {label} during activation");
        }
        app.tools_open = false;
        for _ in 0..20 { frame(&mut app, &context, vec![]); }
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        events.send(DeviceEvent::OpResult { ok: false, message: "transport failure; no automatic retry".into() }).unwrap();
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv(), Ok(crate::device::DeviceCommand::ApplyKeybindings { profile: 2, layer: 1, .. })));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn stopped_worker_reports_unknown_pending_outcome_and_never_retries() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv(), Ok(crate::device::DeviceCommand::ApplyKeybindings { .. })));
        drop(events);
        frame(&mut app, &context, vec![]);
        assert!(app.status_line.contains("outcome is unknown"), "{}", app.status_line);
        assert!(matches!(app.device.status(), DeviceStatus::Error(_)));
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(app.status_line.contains("outcome is unknown"));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn closed_command_queue_is_reported_without_claiming_a_write_was_queued() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        drop(commands);
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(app.status_line.contains("not queued"), "{}", app.status_line);
        assert!(matches!(app.device.status(), DeviceStatus::Error(_)));
    }

    #[test]
    fn lighting_save_includes_both_drafts_from_either_subpage_and_preserves_local_state() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        app.current_layer_mut().unwrap().lighting.rgb = [11, 22, 33];
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        click_label(&mut app, &context, "WASD");
        let expected = serde_json::to_value(&app.current_layer().unwrap().lighting).unwrap();
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        for subpage in ["Base Effect", "Custom Per Key"] {
            click_label(&mut app, &context, subpage);
            let output = click_label(&mut app, &context, "Save to Keyboard");
            assert!(text_shapes(&output).iter().any(|(text, _)| text.contains("Also activates this slot/layer")));
            match commands.try_recv().unwrap() {
                crate::device::DeviceCommand::ApplyLighting { profile, layer, lighting } => {
                    assert_eq!((profile, layer), (2, 1));
                    assert_eq!(serde_json::to_value(lighting).unwrap(), expected);
                }
                other => panic!("unexpected lighting save command: {other:?}"),
            }
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(app.dirty);
            events.send(DeviceEvent::OpResult { ok: true, message: "lighting sent".into() }).unwrap();
            frame(&mut app, &context, vec![]);
        }
    }

    #[test]
    fn hex_input_updates_painted_color_and_invalid_text_blocks_add_but_not_erase() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        click_label(&mut app, &context, "#FF0000");
        frame(&mut app, &context, vec![
            egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false,
                modifiers: egui::Modifiers { command: true, mac_cmd: true, ..Default::default() } },
            egui::Event::Text("#0c2238".into()),
        ]);
        assert_eq!(app.paint_color(), Some([12, 34, 56]));
        click_label(&mut app, &context, "A");
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
        assert_eq!((colors[0].r, colors[0].g, colors[0].b), (12, 34, 56));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        app.paint_hex = "invalid".into();
        click_label(&mut app, &context, "B");
        click_label(&mut app, &context, "WASD");
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        click_label(&mut app, &context, "Erase");
        click_label(&mut app, &context, "A");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().is_empty());
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn painting_all_physical_keys_changes_only_selected_layer_colors_and_round_trips() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        app.paint_hex = "13579B".into();
        app.paint_breathing = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let keys = app.layout.device.matrix_key_buttons.clone();
        for key in &keys { app.paint_keys(&[key]); }
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
        assert_eq!(colors.len(), 99);
        let mut indices: Vec<_> = colors.iter().map(|color| color.matrix_index).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), 99);
        assert!(colors.iter().all(|color| (color.r, color.g, color.b, color.visible) == (19, 87, 155, 254)));
        let exported = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let restored = parse_profile_json(&exported).unwrap();
        assert_eq!(serialize_profile(&restored).unwrap(), exported);
        let mut without_colors = restored;
        without_colors.profiles[2].layers[1].lighting.per_key_colors = None;
        assert_eq!(serialize_profile(&without_colors).unwrap(), before);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn custom_lighting_all_includes_side_leds_preserves_unselected_data_and_requires_save() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        let original = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        app.paint_hex = "#0C2238".into();
        app.paint_breathing = true;
        click_label(&mut app, &context, "All");
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../assets/windows-custom-lighting-vectors.json")).unwrap();
        let expected: Vec<usize> = serde_json::from_value(fixture["groups"][7]["indices"].clone()).unwrap();
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
        let mut indices: Vec<_> = colors.iter().map(|color| color.matrix_index).collect();
        indices.sort_unstable();
        assert_eq!(indices, expected);
        assert!(colors.iter().all(|color| (color.r, color.g, color.b, color.visible) == (12, 34, 56, 254)));
        assert_eq!(colors.len(), 119);
        let mut unchanged = app.local_profile.clone().unwrap();
        unchanged.profiles[2].layers[1].lighting.per_key_colors = None;
        assert_eq!(serialize_profile(&unchanged).unwrap(), original);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        app.dirty = false;
        click_label(&mut app, &context, "All");
        assert!(!app.dirty);
        click_label(&mut app, &context, "Save to Keyboard");
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyLighting { profile, layer, lighting } => {
                assert_eq!((profile, layer), (2, 1));
                assert_eq!(lighting.per_key_colors.as_ref().unwrap().len(), 119);
                lighting.validate().unwrap();
            }
            other => panic!("unexpected command: {other:?}"),
        }
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn custom_lighting_all_erase_preserves_unknown_entries_and_handles_invalid_brush() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let unknown = PerKeyColor { matrix_index: 18, r: 1, g: 2, b: 3, visible: 255 };
        app.current_layer_mut().unwrap().lighting.per_key_colors = Some(vec![unknown.clone()]);
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        app.dirty = false;
        let output = click_label(&mut app, &context, "Side Lights");
        assert!(text_shapes(&output).iter().any(|(text, _)| text == "Quick Key Selection"));
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors, Some(vec![unknown.clone()]));
        assert!(!app.dirty);
        click_label(&mut app, &context, "All");
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().len(), 120);
        app.paint_hex = "invalid".into();
        app.dirty = false;
        click_label(&mut app, &context, "All");
        assert!(!app.dirty);
        click_label(&mut app, &context, "Erase");
        click_label(&mut app, &context, "All");
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors, Some(vec![unknown]));
        app.current_layer_mut().unwrap().lighting.per_key_colors = None;
        app.dirty = false;
        click_label(&mut app, &context, "All");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.is_none());
        assert!(!app.dirty);
        app.paint_hex = "#102030".into();
        click_label(&mut app, &context, "Add");
        click_label(&mut app, &context, "All");
        click_label(&mut app, &context, "Erase");
        click_label(&mut app, &context, "All");
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors, Some(Vec::new()));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn verified_quick_groups_cover_expected_keys_without_duplicates() {
        let (device, _events) = crate::device::tests::isolated_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../assets/windows-custom-lighting-vectors.json")).unwrap();
        assert_eq!(LIGHTING_KEY_GROUPS.len(), 6);
        for ((label, keys), group) in LIGHTING_KEY_GROUPS.iter().zip(fixture["groups"].as_array().unwrap()) {
            assert_eq!(*label, group["label"].as_str().unwrap());
            app.current_layer_mut().unwrap().lighting.per_key_colors = None;
            app.paint_keys(keys);
            let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
            let expected: Vec<usize> = serde_json::from_value(group["indices"].clone()).unwrap();
            let mut indices: Vec<_> = colors.iter().map(|color| color.matrix_index).collect();
            indices.sort_unstable();
            assert_eq!(indices, expected, "group {label}");
            for key in *keys {
                let index = app.layout.led_index_for_matrix_key(key).unwrap();
                assert!(colors.iter().any(|color| color.matrix_index == index));
            }
            app.current_layer().unwrap().validate().unwrap();
        }
    }

    #[test]
    fn painting_replaces_existing_color_and_ignores_noop_or_invalid_edits() {
        let (device, _events) = crate::device::tests::isolated_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.dirty = false;
        app.paint_keys(&["unknown key"]);
        assert!(app.current_layer().unwrap().lighting.per_key_colors.is_none());
        assert!(!app.dirty);
        for invalid in ["", "#fff", "1234567", "-12345", "あいう", "#zzffff"] {
            app.paint_hex = invalid.into();
            assert_eq!(app.paint_color(), None);
            app.paint_keys(&["A"]);
            assert!(!app.dirty);
        }
        app.paint_hex = "#112233".into();
        app.paint_keys(&["A", "A", "D"]);
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().len(), 2);
        app.dirty = false;
        app.paint_keys(&["A", "D"]);
        assert!(!app.dirty);
        app.paint_hex = "#445566".into();
        app.paint_keys(&["A"]);
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
        assert_eq!(colors.len(), 2);
        let index = app.layout.led_index_for_matrix_key("A").unwrap();
        let painted = colors.iter().find(|color| color.matrix_index == index).unwrap();
        assert_eq!((painted.r, painted.g, painted.b), (68, 85, 102));
        app.dirty = false;
        app.erase_lighting = true;
        app.paint_keys(&["B"]);
        assert!(!app.dirty);
        app.paint_keys(&["A"]);
        assert!(app.dirty);
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn custom_lighting_paints_and_erases_a_clicked_key_without_hardware_writes() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        click_label(&mut app, &context, "A");
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().expect("key click should paint the draft");
        assert_eq!(colors.len(), 1);
        assert_eq!(colors[0].matrix_index, app.layout.led_index_for_matrix_key("A").unwrap());
        assert_eq!((colors[0].r, colors[0].g, colors[0].b, colors[0].visible), (255, 0, 0, 255));
        click_label(&mut app, &context, "Erase");
        click_label(&mut app, &context, "A");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().is_empty());
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn breathing_switch_matches_reference_resting_hover_and_focus_geometry() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.custom_lighting = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        let id = egui::Id::new("lighting-breathing");
        let mut centers = Vec::new();
        for (time, enabled) in [(1.0, false), (2.0, true)] {
            app.paint_breathing = enabled;
            frame_at(&mut app, &context, time, vec![]);
            frame_at(&mut app, &context, time + 0.15, vec![]);
            let output = frame_at(&mut app, &context, time + 0.31, vec![]);
            let response = context.read_response(id).expect("reference breathing slider");
            assert_eq!(response.rect.size(), egui::vec2(48.0, 24.0));
            let track = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.rect == response.rect => Some(rect), _ => None,
            }).unwrap();
            assert_eq!(track.fill, egui::Color32::from_gray(17));
            assert_eq!(track.stroke.color, egui::Color32::from_gray(144));
            assert_eq!(track.corner_radius, egui::CornerRadius::same(12));
            let handle = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle) if response.rect.contains(circle.center) && circle.radius == 7.5 => Some(circle), _ => None,
            }).unwrap();
            assert_eq!(handle.stroke.width, 1.0);
            assert_eq!(handle.center.y, response.rect.center().y);
            let color = if enabled { egui::Color32::from_rgb(255, 164, 13) } else { egui::Color32::from_gray(38) };
            assert_eq!(handle.fill, color);
            assert_eq!(handle.stroke.color, if enabled { color } else { egui::Color32::from_gray(217) });
            centers.push(handle.center.x - response.rect.left());
            assert_eq!(binding_text_color(&output, if enabled { "On" } else { "Off" }), egui::Color32::from_gray(217));
            assert_eq!(binding_text_color(&output, "Breathing"), egui::Color32::from_gray(245));
        }
        assert!((centers[1] - centers[0] - 22.0).abs() < 0.1);
        let rect = context.read_response(id).unwrap().rect;
        frame_at(&mut app, &context, 3.0, vec![egui::Event::PointerMoved(rect.center())]);
        let output = frame_at(&mut app, &context, 3.31, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(track)
            if track.rect == rect && track.fill == egui::Color32::from_gray(49)
            && track.stroke.color == egui::Color32::from_rgb(254, 234, 210))));
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Circle(handle)
            if rect.contains(handle.center) && handle.fill == egui::Color32::from_rgb(254, 234, 210))));
        context.memory_mut(|memory| memory.request_focus(id));
        let output = frame_at(&mut app, &context, 3.32, vec![]);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(outline)
            if outline.rect == rect.expand(2.0) && outline.stroke == egui::Stroke::new(2.0_f32, egui::Color32::from_rgb(86, 183, 230)))));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn breathing_switch_changes_only_the_brush_until_keys_are_painted() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.custom_lighting = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        let id = egui::Id::new("lighting-breathing");
        click_label(&mut app, &context, "Breathing");
        assert!(!app.paint_breathing, "the reference text is separate from the clickable switch");
        let rect = context.read_response(id).expect("reference breathing slider").rect;
        click_position(&mut app, &context, rect.center());
        assert!(app.paint_breathing);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        click_label(&mut app, &context, "WASD");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().iter().all(|color| color.visible == 254));
        let painted = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        app.dirty = false;
        context.memory_mut(|memory| memory.request_focus(id));
        for pressed in [true, false] {
            frame(&mut app, &context, vec![egui::Event::Key { key: egui::Key::Space, physical_key: None,
                pressed, repeat: false, modifiers: Default::default() }]);
        }
        assert!(!app.paint_breathing);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), painted);
        assert!(!app.dirty);
        click_label(&mut app, &context, "WASD");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().iter().all(|color| color.visible == 255));
        assert!(app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn quick_group_painting_supports_breathing_and_erase() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        let breathing = context.read_response(egui::Id::new("lighting-breathing")).expect("breathing switch").rect;
        click_position(&mut app, &context, breathing.center());
        click_label(&mut app, &context, "WASD");
        let colors = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap();
        let mut actual: Vec<_> = colors.iter().map(|color| color.matrix_index).collect();
        actual.sort();
        let mut expected: Vec<_> = ["W", "A", "S", "D"].iter().map(|key| app.layout.led_index_for_matrix_key(key).unwrap()).collect();
        expected.sort();
        assert_eq!(actual, expected);
        assert!(colors.iter().all(|color| color.visible == 254));
        click_label(&mut app, &context, "Erase");
        click_label(&mut app, &context, "WASD");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().is_empty());
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn clearing_custom_lighting_only_changes_the_draft_until_explicit_save() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        click_label(&mut app, &context, "Custom Per Key");
        click_label(&mut app, &context, "WASD");
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().len(), 4);
        click_label(&mut app, &context, "Clear custom lighting");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().is_empty());
        assert!(app.dirty);
        click_label(&mut app, &context, "Save to Keyboard");
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        click_label(&mut app, &context, "Remove local override data");
        assert!(app.current_layer().unwrap().lighting.per_key_colors.is_none());
    }

    #[test]
    fn keybinding_save_preserves_the_draft_and_targets_the_selected_slot_and_layer() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "Test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.onboard_slot = 2;
        app.edit_layer = 1;
        let layer = app.current_layer_mut().unwrap();
        layer.key_matrix[60] = 251;
        layer.key_matrix[61] = 193;
        layer.polling_hz = 250;
        layer.lighting.rgb = [11, 22, 33];
        let expected_matrix = layer.key_matrix.clone();
        app.dirty = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        click_label(&mut app, &context, "Keybinding");
        let output = click_label(&mut app, &context, "Save to Keyboard");
        assert!(text_shapes(&output).iter().any(|(text, _)| text.contains("Writes keybindings only")));
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyKeybindings { profile, layer, key_matrix } => {
                assert_eq!((profile, layer), (2, 1));
                assert_eq!(key_matrix, expected_matrix);
            }
            other => panic!("unexpected save command: {other:?}"),
        }
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(app.dirty);
    }

    #[test]
    fn firmware_refresh_requests_only_cached_info_when_connected() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        device.set_status(DeviceStatus::Connected { product: "test keyboard".into() });
        let mut app = GmmkApp::with_device(device);
        app.tools_open = true;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Refresh info");
        assert!(matches!(commands.try_recv(), Ok(crate::device::DeviceCommand::RefreshInfo)));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        assert_eq!(app.firmware, None);
    }

    #[test]
    fn firmware_refresh_is_disabled_when_disconnected() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.tools_open = true;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Refresh info");
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        assert_eq!(app.firmware, None);
    }

    #[test]
    fn firmware_information_is_cleared_after_disconnect_or_device_error() {
        let (mut app, events) = app();
        for status in [DeviceStatus::Disconnected, DeviceStatus::Error("unplugged".into())] {
            events.send(DeviceEvent::Info { fw_version: Some("test-version".into()), message: "info".into() }).unwrap();
            app.poll_device();
            assert_eq!(app.firmware.as_deref(), Some("test-version"));
            events.send(DeviceEvent::Status(status)).unwrap();
            app.poll_device();
            assert_eq!(app.firmware, None);
        }
    }

    #[test]
    fn brightness_text_commits_reference_rounding_only_after_blur() {
        assert_preset_numeric_edits(true, &[
            ("2.49", 0), ("2.5", 5), ("7.5", 10), ("43", 45), ("67", 65),
            ("97.5", 100), ("-10", 0), ("120", 100), ("", 0), ("invalid", 0),
            ("25px", 25), ("1e2", 100), ("1e-", 0), (".5", 0),
            ("Infinity", 100), ("-Infinity", 0), ("0x10", 0), ("  +7.5px", 10),
        ]);
    }

    #[test]
    fn speed_text_commits_absolute_five_point_steps_with_a_minimum_of_one() {
        assert_preset_numeric_edits(false, &[
            ("0", 1), ("1", 1), ("2.49", 1), ("2.5", 5), ("6", 5), ("7.5", 10),
            ("43", 45), ("67", 65), ("97.5", 100), ("120", 100), ("", 1), ("invalid", 1),
        ]);
    }

    fn assert_preset_numeric_edits(brightness: bool, cases: &[(&str, u8)]) {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.select_slot(1);
        app.select_layer(2);
        app.page = Page::Lighting;
        let context = egui::Context::default();
        for (input, expected) in cases {
            let lighting = &mut app.current_layer_mut().unwrap().lighting;
            lighting.effect_ui_value = 1;
            lighting.brightness = 43;
            lighting.speed = 67;
            app.dirty = false;
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            let mut wanted = app.local_profile.clone().unwrap();
            let edited = &mut wanted.profiles[1].layers[2].lighting;
            if brightness { edited.brightness = *expected; } else { edited.speed = *expected; }
            click_label(&mut app, &context, if brightness { "43" } else { "67" });
            frame(&mut app, &context, vec![
                egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false,
                    modifiers: egui::Modifiers { command: true, mac_cmd: true, ..Default::default() } },
                egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed: true, repeat: false,
                    modifiers: Default::default() },
                egui::Event::Text((*input).into()),
            ]);
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before,
                "typing {input:?} must not commit before blur");
            assert!(!app.dirty);
            click_label(&mut app, &context, "Base Effect");
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), serialize_profile(&wanted).unwrap(),
                "blur must commit {input:?} as {expected}");
            assert!(app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn explicit_lighting_save_commits_pending_numeric_text_before_queuing_the_snapshot() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.current_layer_mut().unwrap().lighting.effect_ui_value = 1;
        app.current_layer_mut().unwrap().lighting.speed = 67;
        let context = egui::Context::default();
        click_label(&mut app, &context, "67");
        frame(&mut app, &context, vec![
            egui::Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false,
                modifiers: egui::Modifiers { command: true, mac_cmd: true, ..Default::default() } },
            egui::Event::Text("43".into()),
        ]);
        assert_eq!(app.current_layer().unwrap().lighting.speed, 67);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        click_label(&mut app, &context, "Save to Keyboard");
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyLighting { profile, layer, lighting } => {
                assert_eq!((profile, layer, lighting.speed), (0, 0, 45));
            }
            other => panic!("unexpected save command: {other:?}"),
        }
        assert_eq!(app.current_layer().unwrap().lighting.speed, 45);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn displaying_an_imported_zero_speed_does_not_normalize_or_dirty_the_profile() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        let lighting = &mut app.current_layer_mut().unwrap().lighting;
        lighting.effect_ui_value = 1;
        lighting.speed = 0;
        lighting.brightness = 43;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        for _ in 0..3 { frame(&mut app, &context, vec![]); }
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    fn effect_page_dots(output: &egui::FullOutput) -> Vec<egui::epaint::CircleShape> {
        fn collect(shape: &egui::Shape, circles: &mut Vec<egui::epaint::CircleShape>) {
            match shape {
                egui::Shape::Circle(circle) if circle.radius == 4.0 &&
                    [egui::Color32::from_gray(144), crate::ui::appearance::DEVICE_TAB_ACCENT].contains(&circle.fill) => circles.push(*circle),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes { collect(shape, circles); }
                }
                _ => {}
            }
        }
        let mut circles = Vec::new();
        for shape in &output.shapes { collect(&shape.shape, &mut circles); }
        circles.sort_by(|left, right| left.center.x.total_cmp(&right.center.x));
        circles
    }

    #[test]
    fn inline_color_surface_has_reference_dimensions_without_normalizing_imported_colors() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.current_layer_mut().unwrap().lighting.effect_ui_value = 1;
        app.current_layer_mut().unwrap().lighting.rgb = [254, 255, 254];
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        frame(&mut app, &context, vec![]);
        let scope = egui::Id::new(("preset-color", 0u8, 0u8));
        let saturation = context.read_response(scope.with("saturation")).expect("inline saturation/value control").rect;
        let hue = context.read_response(scope.with("hue")).expect("inline hue control").rect;
        assert_eq!(saturation.height(), 164.0);
        assert_eq!(hue.height(), 24.0);
        assert_eq!(hue.top() - saturation.bottom(), 12.0);
        assert_eq!(saturation.width(), hue.width());
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn keybinding_layers_share_reference_pill_group_at_both_window_sizes() {
        for size in [egui::vec2(1440.0, 880.0), egui::vec2(1000.0, 700.0)] {
            for layer in 0..3 {
                let (device, _events, commands) = crate::device::tests::observed_handle();
                let mut app = GmmkApp::with_device(device);
                app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
                app.select_page(Page::Keybinding);
                app.select_layer(layer);
                let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
                let context = egui::Context::default();
                crate::ui::appearance::configure(&context);
                let mut draw = || context.run(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default()
                }, |context| { crate::ui::appearance::configure(context); app.show(context); });
                draw();
                let output = draw();
                let labels = text_shapes(&output);
                let first = labels.iter().find(|(text, _)| text == "Layer 1").unwrap().1;
                let last = labels.iter().find(|(text, _)| text == "Layer 3").unwrap().1;
                let track = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) if rect.rect.contains_rect(first) && rect.rect.contains_rect(last)
                        && rect.rect.height() == 38.0 && rect.fill == egui::Color32::from_gray(19) => Some(rect),
                    _ => None,
                }).expect("shared three-layer pill group");
                assert_eq!(track.stroke, egui::Stroke::new(1.0_f32, egui::Color32::from_gray(38)));
                assert_eq!(track.corner_radius, egui::CornerRadius::same(19));
                assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(track.rect));
                for index in 0..3 {
                    assert_eq!(binding_text_color(&output, &format!("Layer {}", index + 1)),
                        if index == layer { egui::Color32::from_rgb(26, 16, 3) } else { egui::Color32::from_gray(168) });
                }
                assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
                assert!(!app.dirty);
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
            }
        }
    }

    #[test]
    fn lighting_mode_selector_uses_shared_track_and_reference_colors_at_both_sizes() {
        for size in [egui::vec2(1440.0, 880.0), egui::vec2(1000.0, 700.0)] {
            for custom in [false, true] {
                let (device, _events, commands) = crate::device::tests::observed_handle();
                let mut app = GmmkApp::with_device(device);
                app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
                app.select_page(Page::Lighting);
                app.select_lighting_mode(custom);
                let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
                let context = egui::Context::default();
                crate::ui::appearance::configure(&context);
                let mut draw = || context.run(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default()
                }, |context| { crate::ui::appearance::configure(context); app.show(context); });
                draw();
                let output = draw();
                let labels = text_shapes(&output);
                let base = labels.iter().find(|(text, _)| text == "Base Effect").unwrap().1;
                let per_key = labels.iter().find(|(text, _)| text == "Custom Per Key").unwrap().1;
                let track = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect) if rect.rect.contains_rect(base) && rect.rect.contains_rect(per_key)
                        && rect.rect.height() < 60.0 && rect.fill == egui::Color32::from_gray(28) => Some(rect),
                    _ => None,
                }).expect("one shared lighting-mode track, not separate orange pills");
                assert_eq!(track.corner_radius, egui::CornerRadius::same(3));
                assert_eq!(track.stroke, egui::Stroke::new(1.0_f32, egui::Color32::from_gray(38)));
                assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(track.rect));
                for (label, selected) in [("Base Effect", !custom), ("Custom Per Key", custom)] {
                    assert_eq!(binding_text_color(&output, label), if selected {
                        egui::Color32::from_rgb(217, 206, 244)
                    } else { egui::Color32::from_gray(217) });
                }
                assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
                assert!(!app.dirty);
                assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
            }
        }
    }

    #[test]
    fn lighting_entry_resets_transient_controls_without_changing_drafts_or_pending_save() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.current_layer_mut().unwrap().lighting.rgb = [18, 52, 86];
        app.custom_lighting = true;
        app.paint_hex = "#ABCDEF".into();
        app.paint_breathing = true;
        app.erase_lighting = true;
        app.dirty = true;
        app.pending_write = Some("Saving lighting".into());
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        app.select_page(Page::Lighting);
        assert!(!app.custom_lighting);
        assert!(!app.paint_breathing);
        assert!(!app.erase_lighting);
        assert_eq!(app.paint_color(), Some([18, 52, 86]));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(app.dirty);
        assert_eq!(app.pending_write.as_deref(), Some("Saving lighting"));
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn lighting_mode_handoff_restores_base_color_but_repeated_tab_selection_preserves_brush() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.current_layer_mut().unwrap().lighting.rgb = [18, 52, 86];
        app.select_page(Page::Lighting);
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        click_label(&mut app, &context, "Custom Per Key");
        assert_eq!(app.paint_color(), Some([18, 52, 86]));
        app.paint_hex = "#ABCDEF".into();
        app.paint_breathing = true;
        app.erase_lighting = true;
        click_label(&mut app, &context, "Lighting");
        assert!(app.custom_lighting);
        assert_eq!(app.paint_color(), Some([171, 205, 239]));
        assert!(app.paint_breathing);
        assert!(app.erase_lighting);
        click_label(&mut app, &context, "Base Effect");
        assert_eq!(app.paint_color(), Some([18, 52, 86]));
        click_label(&mut app, &context, "Custom Per Key");
        assert!(app.paint_breathing);
        assert!(app.erase_lighting);
        assert_eq!(app.paint_color(), Some([18, 52, 86]));
        app.paint_hex = "#ABCDEF".into();
        click_label(&mut app, &context, "Custom Per Key");
        assert_eq!(app.paint_color(), Some([171, 205, 239]));
        click_label(&mut app, &context, "My Keyboard");
        click_label(&mut app, &context, "Lighting");
        assert!(!app.custom_lighting);
        assert!(!app.paint_breathing);
        assert!(!app.erase_lighting);
        assert_eq!(app.paint_color(), Some([18, 52, 86]));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn inline_preset_color_edits_only_selected_rgb_and_requires_explicit_save() {
        let (device, events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        events.send(DeviceEvent::Status(DeviceStatus::Connected { product: "Test keyboard".into() })).unwrap();
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.select_slot(1);
        app.select_layer(2);
        app.page = Page::Lighting;
        app.current_layer_mut().unwrap().lighting.effect_ui_value = 1;
        app.current_layer_mut().unwrap().lighting.rgb = [255, 0, 0];
        let mut expected = app.local_profile.clone().unwrap();
        expected.profiles[1].layers[2].lighting.rgb = [128, 64, 64];
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        frame(&mut app, &context, vec![]);
        let scope = egui::Id::new(("preset-color", 1u8, 2u8));
        let response = context.read_response(scope.with("saturation")).expect("inline color control");
        click_position(&mut app, &context, response.rect.center());
        assert_eq!(app.paint_color(), Some([128, 64, 64]));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), serialize_profile(&expected).unwrap());
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        click_label(&mut app, &context, "Save to Keyboard");
        match commands.try_recv().unwrap() {
            crate::device::DeviceCommand::ApplyLighting { profile, layer, lighting } => assert_eq!((profile, layer, lighting.rgb), (1, 2, [128, 64, 64])),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn inline_custom_color_changes_only_the_brush_until_a_key_is_painted() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.custom_lighting = true;
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        frame(&mut app, &context, vec![]);
        let scope = egui::Id::new(("custom-color", 0u8, 0u8));
        let response = context.read_response(scope.with("hue")).expect("inline hue control");
        click_position(&mut app, &context, response.rect.left_center() + egui::vec2(response.rect.width() / 3.0, 0.0));
        assert_eq!(app.paint_color(), Some([0, 255, 0]));
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        click_label(&mut app, &context, "WASD");
        for key in ["W", "A", "S", "D"] {
            let address = app.layout.led_index_for_matrix_key(key).unwrap();
            let color = app.current_layer().unwrap().lighting.per_key_colors.as_ref().unwrap().iter()
                .find(|entry| entry.matrix_index == address).unwrap();
            assert_eq!([color.r, color.g, color.b], [0, 255, 0]);
        }
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn quick_selection_grid_matches_reference_geometry_and_stays_below_keyboard() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../assets/windows-custom-lighting-vectors.json")).unwrap();
        for size in [egui::vec2(1440.0, 880.0), egui::vec2(1000.0, 700.0)] {
            let (device, _events, commands) = crate::device::tests::observed_handle();
            let mut app = GmmkApp::with_device(device);
            app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
            app.page = Page::Lighting;
            app.custom_lighting = true;
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            let context = egui::Context::default();
            crate::ui::appearance::configure(&context);
            let mut render = || context.run(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)), ..Default::default()
            }, |context| app.show(context));
            render();
            let output = render();
            let first = binding_tile_shape(&output, "WASD").rect;
            let heading = text_shapes(&output).into_iter().find(|(text, _)| text == "Quick Key Selection").unwrap().1;
            let hex = text_shapes(&output).into_iter().find(|(text, _)| text == "HEX").unwrap().1;
            assert!(first.left() > hex.right(), "quick controls belong below the right-hand preview");
            assert!((first.left() - heading.left() - 8.0).abs() < 1.0);
            assert_eq!(binding_text_color(&output, "Quick Key Selection"), egui::Color32::from_gray(245));
            for (index, group) in fixture["groups"].as_array().unwrap().iter().enumerate() {
                let label = group["label"].as_str().unwrap();
                let shape = binding_tile_shape(&output, label);
                assert_eq!(shape.rect.size(), egui::vec2(110.0, 32.0), "{label}, {size:?}");
                assert_eq!(shape.rect.min - first.min, egui::vec2((index % 4) as f32 * 125.0, (index / 4) as f32 * 47.0));
                assert_eq!(shape.corner_radius, egui::CornerRadius::same(16));
                assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(shape.rect));
                if label != "Side Lights" {
                    assert_eq!(shape.fill, egui::Color32::from_gray(38));
                    assert_eq!(binding_text_color(&output, label), egui::Color32::from_gray(217));
                }
                let save = text_shapes(&output).into_iter().find(|(text, _)| text == "Save to Keyboard").unwrap().1;
                assert!(shape.rect.bottom() < save.top());
            }
            let keys: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
                egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_gray(25) => Some(rect.rect),
                _ => None,
            }).collect();
            assert_eq!(keys.len(), 99);
            assert!(keys.iter().all(|key| key.bottom() < heading.top()));
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(!app.dirty);
            assert!(commands.try_recv().is_err());
        }
    }

    #[test]
    fn quick_selection_keyboard_activation_respects_group_scope_and_disabled_actions() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../assets/windows-custom-lighting-vectors.json")).unwrap();
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        app.custom_lighting = true;
        let context = egui::Context::default();
        frame(&mut app, &context, vec![]);
        for (index, group) in fixture["groups"].as_array().unwrap().iter().enumerate() {
            app.current_layer_mut().unwrap().lighting.per_key_colors = None;
            app.dirty = false;
            let id = egui::Id::new("lighting-quick-keys").with(index);
            let response = context.read_response(id).expect("quick selector response");
            assert_eq!(response.sense.senses_click(), index != 6);
            context.memory_mut(|memory| memory.request_focus(id));
            for pressed in [true, false] {
                frame(&mut app, &context, vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None,
                    pressed, repeat: false, modifiers: Default::default() }]);
            }
            let mut actual: Vec<_> = app.current_layer().unwrap().lighting.per_key_colors.as_deref().unwrap_or_default()
                .iter().map(|color| color.matrix_index).collect();
            actual.sort_unstable();
            let expected: Vec<usize> = serde_json::from_value(group["indices"].clone()).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(app.dirty, index != 6);
        }
        app.paint_hex = "invalid".into();
        app.dirty = false;
        frame(&mut app, &context, vec![]);
        frame(&mut app, &context, vec![]);
        let id = egui::Id::new("lighting-quick-keys").with(7usize);
        assert!(!context.read_response(id).unwrap().sense.senses_click(), "brush={}, erase={}", app.paint_hex, app.erase_lighting);
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        context.memory_mut(|memory| memory.request_focus(id));
        for pressed in [true, false] {
            frame(&mut app, &context, vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None,
                pressed, repeat: false, modifiers: Default::default() }]);
        }
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        app.erase_lighting = true;
        frame(&mut app, &context, vec![]);
        frame(&mut app, &context, vec![]);
        assert!(context.read_response(id).unwrap().sense.senses_click());
        assert!(!app.dirty);
        context.memory_mut(|memory| memory.request_focus(id));
        for pressed in [true, false] {
            frame(&mut app, &context, vec![egui::Event::Key { key: egui::Key::Space, physical_key: None,
                pressed, repeat: false, modifiers: Default::default() }]);
        }
        assert_eq!(app.current_layer().unwrap().lighting.per_key_colors, Some(Vec::new()));
        assert!(app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn quick_selection_is_visible_only_for_an_imported_custom_lighting_draft() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let context = egui::Context::default();
        app.custom_lighting = true;
        for page in Page::ALL {
            app.page = page;
            let output = frame(&mut app, &context, vec![]);
            assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Quick Key Selection"));
        }
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        for page in Page::ALL {
            for custom in [false, true] {
                app.page = page;
                app.custom_lighting = custom;
                frame(&mut app, &context, vec![]);
                let output = frame(&mut app, &context, vec![]);
                let visible = text_shapes(&output).iter().any(|(text, _)| text == "Quick Key Selection");
                assert_eq!(visible, page == Page::Lighting && custom);
            }
        }
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
        assert!(!app.dirty);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn preset_effect_tiles_match_reference_geometry_and_colors_at_supported_window_sizes() {
        for size in [egui::vec2(1440.0, 880.0), egui::vec2(1000.0, 700.0)] {
            let (device, _events, commands) = crate::device::tests::observed_handle();
            let mut app = GmmkApp::with_device(device);
            app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
            app.current_layer_mut().unwrap().lighting.effect_ui_value = 8;
            app.page = Page::Lighting;
            let context = egui::Context::default();
            crate::ui::appearance::configure(&context);
            let mut render = || context.run(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            }, |context| app.show(context));
            render();
            let output = render();
            let first = binding_tile_shape(&output, "LED Off").rect;
            for (index, label) in ["LED Off", "Glorious Mode", "Wave #1", "Wave #2", "Spiraling Wave",
                "Acid Mode", "Breathing", "Normally On", "Ripple Graff"].into_iter().enumerate() {
                let tile = binding_tile_shape(&output, label);
                assert_eq!(tile.rect.size(), egui::vec2(120.0, 40.0), "{label} at {size:?}");
                assert_eq!(tile.rect.min - first.min, egui::vec2((index % 3) as f32 * 132.0, (index / 3) as f32 * 52.0));
                assert_eq!(tile.corner_radius, egui::CornerRadius::same(3));
                assert_eq!(tile.fill, if index == 0 { egui::Color32::from_rgb(197, 180, 227) } else { egui::Color32::from_gray(38) });
                assert_eq!(tile.stroke, egui::Stroke::new(1.0_f32, egui::Color32::from_gray(81)));
                assert_eq!(binding_text_color(&output, label), if index == 0 { egui::Color32::from_gray(28) } else { egui::Color32::from_gray(191) });
            }
            let dots = effect_page_dots(&output);
            assert_eq!(dots.len(), 2);
            assert_eq!(dots[1].center.x - dots[0].center.x, 16.0);
            assert_eq!(dots[0].fill, crate::ui::appearance::DEVICE_TAB_ACCENT);
            assert!(!app.dirty);
            assert!(commands.try_recv().is_err());
        }
    }

    #[test]
    fn preset_pagination_and_all_eighteen_choices_preserve_other_draft_fields_without_writing() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.select_slot(1);
        app.select_layer(2);
        app.current_layer_mut().unwrap().lighting.effect_ui_value = 8;
        app.page = Page::Lighting;
        let context = egui::Context::default();
        let original = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let labels = ["LED Off", "Glorious Mode", "Wave #1", "Wave #2", "Spiraling Wave", "Acid Mode", "Breathing",
            "Normally On", "Ripple Graff", "Pass Without Trace", "Fast Run Without Trace", "Matrix 2", "Matrix 3",
            "Rainbow", "Heartbeat Sensor", "Digital Times", "Kamehemeha", "Pingpong"];
        for (page, choices) in labels.chunks(9).enumerate() {
            frame(&mut app, &context, vec![]);
            let output = frame(&mut app, &context, vec![]);
            let dots = effect_page_dots(&output);
            assert_eq!(dots.len(), 2, "two dot buttons, not three text pages");
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            let dirty = app.dirty;
            let output = click_position(&mut app, &context, dots[page].center);
            assert_eq!(app.effect_page, page);
            assert_eq!(app.dirty, dirty);
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Surmount"));
            for (index, label) in choices.iter().enumerate() {
                click_label(&mut app, &context, label);
                let effect = LED_EFFECT_IDS[page * 9 + index];
                let mut expected = parse_profile_json(&original).unwrap();
                expected.profiles[1].layers[2].lighting.effect_ui_value = effect;
                assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), serialize_profile(&expected).unwrap());
                assert!(commands.try_recv().is_err());
            }
        }
        for (name, expected) in [("preset-effect-next", 1), ("preset-effect-previous", 0), ("preset-effect-previous", 0)] {
            let response = context.read_response(egui::Id::new(name)).expect("arrow button");
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            click_position(&mut app, &context, response.rect.center());
            assert_eq!(app.effect_page, expected);
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(commands.try_recv().is_err());
        }
    }

    #[test]
    fn preset_import_selects_its_reference_page_without_normalizing_hidden_surmount() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        let template = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json")).unwrap();
        let path = std::env::temp_dir().join(format!("gmmk-effect-import-{}.json", std::process::id()));
        for (index, effect) in LED_EFFECT_IDS.iter().enumerate() {
            let mut profile = parse_profile_json(&template).unwrap();
            profile.profiles[profile.active_profile as usize].layers[0].lighting.effect_ui_value = *effect;
            let json = serialize_profile(&profile).unwrap();
            std::fs::write(&path, &json).unwrap();
            app.import_path(path.clone());
            std::fs::remove_file(&path).unwrap();
            assert_eq!(app.effect_page, index / 9, "imported effect {effect}");
            app.page = Page::Lighting;
            let context = egui::Context::default();
            frame(&mut app, &context, vec![]);
            let output = frame(&mut app, &context, vec![]);
            if *effect == 18 {
                assert!(!text_shapes(&output).iter().any(|(text, _)| text == "Surmount"));
                assert!(effect_page_dots(&output).iter().all(|dot| dot.fill == egui::Color32::from_gray(144)));
            }
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), json);
            assert!(!app.dirty);
            assert!(commands.try_recv().is_err());
        }
    }

    #[test]
    fn reopening_base_effects_returns_to_the_selected_effect_page_without_changing_the_draft() {
        let (mut app, _events) = app();
        app.current_layer_mut().unwrap().lighting.effect_ui_value = 17;
        let context = egui::Context::default();
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        let output = click_label(&mut app, &context, "Lighting");
        assert_eq!(app.effect_page, 1);
        let dots = effect_page_dots(&output);
        click_position(&mut app, &context, dots[0].center);
        assert_eq!(app.effect_page, 0);
        click_label(&mut app, &context, "Custom Per Key");
        click_label(&mut app, &context, "Base Effect");
        assert_eq!(app.effect_page, 1);
        let output = frame(&mut app, &context, vec![]);
        click_position(&mut app, &context, effect_page_dots(&output)[0].center);
        click_label(&mut app, &context, "My Keyboard");
        click_label(&mut app, &context, "Lighting");
        assert_eq!(app.effect_page, 1);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
    }

    #[test]
    fn preset_lighting_controls_follow_reference_effect_availability_without_mutating_drafts() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        app.page = Page::Lighting;
        let context = egui::Context::default();
        let cases = [
            (8, false, false, false),
            (0, true, true, false),
            (1, true, true, true),
            (3, true, true, true),
            (4, true, true, true),
            (5, true, true, false),
            (2, true, true, true),
            (6, true, false, true),
            (7, true, true, true),
            (9, true, true, true),
            (10, true, true, true),
            (11, true, true, true),
            (12, true, true, true),
            (13, true, true, false),
            (14, true, true, true),
            (15, true, true, true),
            (16, true, true, true),
            (17, true, true, true),
            (18, true, true, true),
        ];
        for (index, (effect, brightness, speed, color)) in cases.into_iter().enumerate() {
            assert_eq!(effect, LED_EFFECT_IDS[index]);
            app.effect_page = index / 9;
            let lighting = &mut app.current_layer_mut().unwrap().lighting;
            lighting.effect_ui_value = effect;
            lighting.brightness = 43;
            lighting.speed = 67;
            lighting.rgb = [11, 22, 33];
            app.dirty = false;
            let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
            frame(&mut app, &context, vec![]);
            let labels = lighting_control_labels(&mut app, &context);
            for (label, expected) in [("Brightness", brightness), ("Speed", speed), ("Effect Color", color)] {
                assert_eq!(labels.contains(&label), expected, "effect {effect}: {label}");
            }
            let expected_order: Vec<_> = [("Effect Color", color), ("Brightness", brightness), ("Speed", speed)]
                .into_iter().filter_map(|(label, visible)| visible.then_some(label)).collect();
            assert_eq!(labels, expected_order, "effect {effect}: preset control order");
            assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
            assert!(!app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
    }

    #[test]
    fn changing_lighting_effects_hides_controls_without_resetting_their_values_or_writing() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let lighting = &mut app.current_layer_mut().unwrap().lighting;
        lighting.effect_ui_value = 1;
        lighting.brightness = 43;
        lighting.speed = 67;
        lighting.rgb = [11, 22, 33];
        let original = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        app.dirty = false;
        let context = egui::Context::default();
        click_label(&mut app, &context, "Lighting");
        for (label, effect, brightness, speed, color) in [
            ("LED Off", 8, false, false, false),
            ("Normally On", 6, true, false, true),
            ("Acid Mode", 5, true, true, false),
            ("Breathing", 2, true, true, true),
            ("Wave #1", 1, true, true, true),
        ] {
            click_label(&mut app, &context, label);
            let labels = lighting_control_labels(&mut app, &context);
            for (control, expected) in [("Brightness", brightness), ("Speed", speed), ("Effect Color", color)] {
                assert_eq!(labels.contains(&control), expected, "{label}: {control}");
            }
            let lighting = &app.current_layer().unwrap().lighting;
            assert_eq!(lighting.effect_ui_value, effect);
            assert_eq!((lighting.brightness, lighting.speed, lighting.rgb), (43, 67, [11, 22, 33]));
            assert!(app.dirty);
            assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
        }
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), original);
    }

    #[test]
    fn performance_and_effect_controls_edit_only_the_local_selected_layer() {
        let (device, _events, commands) = crate::device::tests::observed_handle();
        let mut app = GmmkApp::with_device(device);
        app.import_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let context = egui::Context::default();
        let matrix = app.current_layer().unwrap().key_matrix.clone();
        click_label(&mut app, &context, "Performance");
        click_label(&mut app, &context, "1000 Hz");
        click_label(&mut app, &context, "125 Hz");
        assert_eq!(app.current_layer().unwrap().polling_hz, 125);
        click_label(&mut app, &context, "16 ms");
        click_label(&mut app, &context, "2 ms");
        assert_eq!(app.current_layer().unwrap().lighting.input_latency_ms, 2);
        click_label(&mut app, &context, "Lighting");
        let output = frame(&mut app, &context, vec![]);
        click_position(&mut app, &context, effect_page_dots(&output)[1].center);
        click_label(&mut app, &context, "Pingpong");
        assert_eq!(app.current_layer().unwrap().lighting.effect_ui_value, 17);
        assert_eq!(app.current_layer().unwrap().key_matrix, matrix);
        assert!(app.dirty);
        assert!(matches!(commands.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn cancelling_import_preserves_unexported_changes() {
        let (mut app, _events) = app();
        let context = egui::Context::default();
        app.pending_file = Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/factory_profile.gmmkprofile.json"));
        let before = serialize_profile(app.local_profile.as_ref().unwrap()).unwrap();
        click_label(&mut app, &context, "Keep current draft");
        assert!(app.pending_file.is_none());
        assert!(app.dirty);
        assert_eq!(serialize_profile(app.local_profile.as_ref().unwrap()).unwrap(), before);
    }
}


impl eframe::App for GmmkApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }
}
