mod read;
mod discovery;
mod transport;

pub use read::{FIRMWARE_INFO_HELP, ONBOARD_READ_UNAVAILABLE};

use crate::protocol::{
    build_change_profile, build_key_matrix, build_led_effect,
    build_led_layout_chunk, build_led_type, build_polling_rate, PID,
    PID_BOOTLOADER_US, REPORT_SIZE, USAGE_CONFIG, USAGE_PAGE_CONFIG, VID,
};
use crate::profile::{LayerState, LightingState};
use std::ffi::{CStr, CString};
use hidapi::{HidApi, HidDevice};
use parking_lot::Mutex;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

const POST_WRITE_DELAY_MS: u64 = 120;
const NOTIFICATION_POLL_INTERVAL: Duration = Duration::from_millis(20);
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum DeviceStatus {
    Disconnected,
    Connected { product: String },
    Error(String),
}

#[derive(Debug)]
pub enum DeviceCommand {
    Connect,
    Disconnect,
    RefreshInfo,
    SwitchProfile { profile: u8, layer: u8 },
    ApplyKeybindings {
        profile: u8,
        layer: u8,
        key_matrix: Vec<u8>,
    },
    ApplyLighting {
        profile: u8,
        layer: u8,
        lighting: LightingState,
    },
    ApplyLayer {
        profile: u8,
        layer: u8,
        layer_state: LayerState,
    },
    ApplyFullProfile {
        profile: u8,
        layers: [LayerState; 3],
    },
}

#[derive(Debug, Clone)]
pub enum DeviceEvent {
    Status(DeviceStatus),
    Info { fw_version: Option<String>, message: String },
    OpResult { ok: bool, message: String },
    NotificationStatus { listening: bool, message: String },
    ProfileNotification { profile: u8, layer: u8 },
}

pub struct DeviceHandle {
    cmd_tx: Sender<DeviceCommand>,
    pub events: Receiver<DeviceEvent>,
    status: Mutex<DeviceStatus>,
}

impl DeviceHandle {
    pub fn spawn() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        thread::spawn(move || worker(cmd_rx, evt_tx));
        Self {
            cmd_tx,
            events: evt_rx,
            status: Mutex::new(DeviceStatus::Disconnected),
        }
    }

    pub fn connect(&self) {
        let _ = self.cmd_tx.send(DeviceCommand::Connect);
    }

    pub fn disconnect(&self) {
        let _ = self.cmd_tx.send(DeviceCommand::Disconnect);
    }

    pub fn refresh_info(&self) {
        let _ = self.cmd_tx.send(DeviceCommand::RefreshInfo);
    }

    pub fn switch_profile(&self, profile: u8, layer: u8) -> Result<(), String> {
        self.queue_write(DeviceCommand::SwitchProfile { profile, layer })
    }

    pub fn apply_layer(&self, profile: u8, layer: u8, layer_state: LayerState) -> Result<(), String> {
        self.queue_write(DeviceCommand::ApplyLayer {
            profile,
            layer,
            layer_state,
        })
    }

    pub fn apply_keybindings(&self, profile: u8, layer: u8, key_matrix: Vec<u8>) -> Result<(), String> {
        self.queue_write(DeviceCommand::ApplyKeybindings { profile, layer, key_matrix })
    }

    pub fn apply_lighting(&self, profile: u8, layer: u8, lighting: LightingState) -> Result<(), String> {
        self.queue_write(DeviceCommand::ApplyLighting { profile, layer, lighting })
    }

    pub fn apply_full_profile(&self, profile: u8, layers: [LayerState; 3]) -> Result<(), String> {
        self.queue_write(DeviceCommand::ApplyFullProfile { profile, layers })
    }

    fn queue_write(&self, command: DeviceCommand) -> Result<(), String> {
        self.cmd_tx.send(command).map_err(|_| "Device worker unavailable; write was not queued. Restart the app.".into())
    }

    pub fn set_status(&self, s: DeviceStatus) {
        *self.status.lock() = s;
    }

    pub fn status(&self) -> DeviceStatus {
        self.status.lock().clone()
    }
}

struct WorkerState {
    api: HidApi,
    device: Option<HidDevice>,
    notifications: Option<HidDevice>,
    product: String,
    release_number: Option<u16>,
}

fn worker(cmd_rx: Receiver<DeviceCommand>, evt_tx: Sender<DeviceEvent>) {
    let mut state = match HidApi::new() {
        Ok(api) => WorkerState {
            api,
            device: None,
            notifications: None,
            product: String::new(),
            release_number: None,
        },
        Err(e) => {
            let _ = evt_tx.send(DeviceEvent::Status(DeviceStatus::Error(e.to_string())));
            return;
        }
    };

    let mut discovery = discovery::Discovery::default();
    let mut next_scan = Instant::now();
    loop {
        let command = match cmd_rx.recv_timeout(NOTIFICATION_POLL_INTERVAL) {
            Ok(command) => Some(command),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        if let Some(cmd) = command { match cmd {
            DeviceCommand::Connect => {
                discovery.retry();
                next_scan = Instant::now();
            },
            DeviceCommand::Disconnect => {
                discovery.pause();
                state.device = None;
                state.notifications = None;
                state.release_number = None;
                let _ = evt_tx.send(DeviceEvent::Status(DeviceStatus::Disconnected));
            }
            DeviceCommand::RefreshInfo => {
                let _ = evt_tx.send(cached_firmware_info(state.release_number));
            }
            DeviceCommand::SwitchProfile { profile, layer } => {
                let result = with_device(&mut state, |dev| {
                    activate_profile(profile, layer, &mut |report, delay| {
                        send_report(dev, report, delay).map(|_| ())
                    })
                });
                let _ = evt_tx.send(DeviceEvent::OpResult {
                    ok: result.is_ok(),
                    message: result.unwrap_or_else(|e| e),
                });
            }
            DeviceCommand::ApplyKeybindings { profile, layer, key_matrix } => {
                let result = with_device(&mut state, |dev| {
                    apply_keybindings(profile, layer, &key_matrix, &mut |report, delay| {
                        send_report(dev, report, delay).map(|_| ())
                    })
                });
                let _ = evt_tx.send(DeviceEvent::OpResult {
                    ok: result.is_ok(),
                    message: result.unwrap_or_else(|e| e),
                });
            }
            DeviceCommand::ApplyLighting { profile, layer, lighting } => {
                let result = with_device(&mut state, |dev| {
                    apply_lighting(profile, layer, &lighting, &mut |report, delay| {
                        send_report(dev, report, delay).map(|_| ())
                    })
                });
                let _ = evt_tx.send(DeviceEvent::OpResult {
                    ok: result.is_ok(),
                    message: result.unwrap_or_else(|e| e),
                });
            }
            DeviceCommand::ApplyLayer {
                profile,
                layer,
                layer_state,
            } => {
                let result = with_device(&mut state, |dev| {
                    apply_layer_to_device(profile, layer, &layer_state, &mut |report| {
                        send_report(dev, report, Duration::from_millis(POST_WRITE_DELAY_MS)).map(|_| ())
                    })
                });
                let _ = evt_tx.send(DeviceEvent::OpResult {
                    ok: result.is_ok(),
                    message: result.unwrap_or_else(|e| e),
                });
            }
            DeviceCommand::ApplyFullProfile { profile, layers } => {
                let result = with_device(&mut state, |dev| {
                    apply_full_profile(profile, &layers, &mut |report| {
                        send_report(dev, report, Duration::from_millis(POST_WRITE_DELAY_MS)).map(|_| ())
                    })
                });
                let _ = evt_tx.send(DeviceEvent::OpResult {
                    ok: result.is_ok(),
                    message: result.unwrap_or_else(|e| e),
                });
            }
        } }
        if Instant::now() >= next_scan {
            discover_device(&mut state, &mut discovery, &evt_tx);
            next_scan = Instant::now() + DISCOVERY_INTERVAL;
        }
        poll_profile_notifications(&mut state, &evt_tx);
    }
}

fn discover_device(state: &mut WorkerState, discovery: &mut discovery::Discovery, events: &Sender<DeviceEvent>) {
    let observed = state.api.refresh_devices().map_err(|error| error.to_string()).and_then(|()| {
        unique_interface_path(state.api.device_list().filter(|device| {
            device.vendor_id() == VID && device.product_id() == PID
                && device.usage_page() == USAGE_PAGE_CONFIG && device.usage() == USAGE_CONFIG
        }).map(|device| device.path()))
    });
    let status = match discovery.observe(observed) {
        discovery::Change::Unchanged => return,
        discovery::Change::Present => match open_device(state) {
            Ok(product) => {
                state.product = product.clone();
                let _ = events.send(DeviceEvent::Status(DeviceStatus::Connected { product }));
                let _ = events.send(cached_firmware_info(state.release_number));
                let _ = events.send(open_profile_notifications(state));
                return;
            }
            Err(error) => DeviceStatus::Error(error),
        },
        discovery::Change::Absent => DeviceStatus::Disconnected,
        discovery::Change::Failed(error) => DeviceStatus::Error(error),
    };
    state.device = None;
    state.notifications = None;
    state.release_number = None;
    let _ = events.send(DeviceEvent::Status(status));
}

fn unique_interface_path<'a>(paths: impl Iterator<Item = &'a CStr>) -> Result<Option<CString>, String> {
    let mut selected: Option<CString> = None;
    for path in paths {
        match &selected {
            Some(previous) if previous.as_c_str() != path => {
                return Err("Multiple matching keyboard interfaces; disconnect other matching keyboards before connecting".into());
            }
            None => selected = Some(path.into()),
            _ => {}
        }
    }
    Ok(selected)
}

fn open_profile_notifications(state: &mut WorkerState) -> DeviceEvent {
    let result = unique_interface_path(state.api.device_list().filter(|device| {
        device.vendor_id() == VID && device.product_id() == PID
            && device.usage_page() == 0xff00 && device.usage() == 0xff00
    }).map(|device| device.path()))
        .and_then(|path| path.ok_or_else(|| "Notification interface 0xFF00/0xFF00 not found".into()))
        .and_then(|path| state.api.open_path(&path).map_err(|error| error.to_string()));
    match result {
        Ok(device) => {
            state.notifications = Some(device);
            DeviceEvent::NotificationStatus {
                listening: true,
                message: "Listening for slot/layer notifications; none received yet. No feature-report query or notification-enable write is sent.".into(),
            }
        }
        Err(error) => DeviceEvent::NotificationStatus {
            listening: false,
            message: format!("Slot/layer notifications unavailable: {error}. Configuration connection remains open."),
        },
    }
}

fn poll_profile_notifications(state: &mut WorkerState, events: &Sender<DeviceEvent>) {
    let Some(device) = &state.notifications else { return };
    match read::read_profile_notifications(&mut |buffer| {
        device.read_timeout(buffer, 0).map_err(|error| error.to_string())
    }) {
        Ok(Some((profile, layer))) => { let _ = events.send(DeviceEvent::ProfileNotification { profile, layer }); }
        Ok(None) => {}
        Err(error) => {
            state.notifications = None;
            let _ = events.send(DeviceEvent::NotificationStatus {
                listening: false,
                message: format!("Slot/layer notification reader stopped: {error}. No automatic retry; reconnect to listen again."),
            });
        }
    }
}

fn open_device(state: &mut WorkerState) -> Result<String, String> {
    state.device = None;
    state.notifications = None;
    state.release_number = None;
    state.api.refresh_devices().map_err(|e| e.to_string())?;
    let found_path = unique_interface_path(state.api.device_list().filter(|device| {
        device.vendor_id() == VID && device.product_id() == PID
            && device.usage_page() == USAGE_PAGE_CONFIG && device.usage() == USAGE_CONFIG
    }).map(|device| device.path()))?;
    let mut product = String::new();
    let mut release_number = None;
    for dev in state.api.device_list() {
        if dev.vendor_id() != VID || dev.product_id() != PID {
            continue;
        }
        if dev.product_id() == PID_BOOTLOADER_US {
            return Err("keyboard is in bootloader mode; refusing to configure".into());
        }
        if dev.usage_page() == USAGE_PAGE_CONFIG && dev.usage() == USAGE_CONFIG {
            release_number = Some(dev.release_number());
            product = dev
                .product_string()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "GMMK 2 96%".into());
            break;
        }
    }
    let path = found_path.ok_or_else(|| {
        String::from(
            "GMMK config interface not found (0x320F:0x504B, usage 0xFF01/0x01). Close Glorious CORE.",
        )
    })?;
    let device = state
        .api
        .open_path(&path)
        .map_err(|e| format!("open failed: {e}"))?;
    let mut descriptor = [0; 4096];
    let length = device.get_report_descriptor(&mut descriptor)
        .map_err(|error| format!("Cannot verify configuration HID descriptor: {error}. No settings sent."))?;
    transport::validate_descriptor(&descriptor[..length])?;
    state.device = Some(device);
    state.release_number = release_number;
    Ok(product)
}

fn cached_firmware_info(release_number: Option<u16>) -> DeviceEvent {
    let fw_version = release_number.and_then(read::firmware_version_from_release);
    let message = match release_number {
        Some(raw) => format!("Cached USB release {raw:04X}. {FIRMWARE_INFO_HELP}"),
        None => "No connected device metadata. No query sent.".into(),
    };
    DeviceEvent::Info { fw_version, message }
}

fn with_device<F, T>(state: &mut WorkerState, f: F) -> Result<T, String>
where
    F: FnOnce(&mut HidDevice) -> Result<T, String>,
{
    let dev = state
        .device
        .as_mut()
        .ok_or_else(|| "not connected".to_string())?;
    f(dev)
}

fn send_report(dev: &mut HidDevice, buf: &[u8], delay_after: Duration) -> Result<String, String> {
    let report = transport::feature_report(buf)?;
    dev.set_blocking_mode(true).map_err(|e| e.to_string())?;
    dev.send_feature_report(&report)
        .map_err(|e| format!("SetFeatureReport failed (report {}, command {}, {} bytes): {e}", report[0], report[1], report.len()))?;
    thread::sleep(delay_after);
    Ok(format!("Wrote {} bytes", report.len()))
}

fn apply_layer_to_device(
    profile: u8,
    layer: u8,
    layer_state: &LayerState,
    send: &mut impl FnMut(&[u8; REPORT_SIZE]) -> Result<(), String>,
) -> Result<String, String> {
    let reports = layer_reports(profile, layer, layer_state)?;
    send_reports(&reports, send)?;
    Ok(format!(
        "Sent profile {} layer {} to keyboard (not read back)",
        profile + 1,
        layer + 1
    ))
}

fn apply_keybindings(
    profile: u8,
    layer: u8,
    key_matrix: &[u8],
    send: &mut impl FnMut(&[u8], Duration) -> Result<(), String>,
) -> Result<String, String> {
    let report = build_key_matrix(profile, layer, key_matrix)?;
    send_reports(&[report], &mut |report| send(report, Duration::from_millis(150)))?;
    Ok(format!(
        "Sent keybindings only to profile {} layer {} (not read back)",
        profile + 1,
        layer + 1
    ))
}

fn activate_profile(
    profile: u8,
    layer: u8,
    send: &mut impl FnMut(&[u8], Duration) -> Result<(), String>,
) -> Result<String, String> {
    let report = build_change_profile(profile, layer);
    send_reports(&[report], &mut |report| send(report, Duration::from_millis(550)))?;
    Ok(format!(
        "Sent activation for profile {} layer {} (not read back)",
        profile + 1,
        layer + 1
    ))
}

fn apply_lighting(
    profile: u8,
    layer: u8,
    lighting: &LightingState,
    send: &mut impl FnMut(&[u8], Duration) -> Result<(), String>,
) -> Result<String, String> {
    let lighting_frames = lighting_reports(profile, layer, lighting)?;
    let activation = build_change_profile(profile, layer).to_vec();
    let mut reports = Vec::new();
    for report in lighting_frames {
        reports.push(report.to_vec());
        if report[1] == 2 {
            reports.push(activation.clone());
        }
    }
    if lighting.per_key_colors.is_some() {
        reports.push(activation);
    }
    send_reports(&reports, &mut |report| {
        let delay = if report[1] == 1 { 550 } else { 100 };
        send(report, Duration::from_millis(delay))
    })?;
    Ok(format!(
        "Sent lighting, input latency and activation for profile {} layer {} (not read back)",
        profile + 1,
        layer + 1
    ))
}

fn apply_full_profile(
    profile: u8,
    layers: &[LayerState; 3],
    send: &mut impl FnMut(&[u8; REPORT_SIZE]) -> Result<(), String>,
) -> Result<String, String> {
    let mut reports = Vec::new();
    for (layer, layer_state) in layers.iter().enumerate() {
        reports.extend(layer_reports(profile, layer as u8, layer_state)?);
    }
    send_reports(&reports, send)?;
    Ok(format!(
        "Sent all layers to onboard profile {} (not read back)",
        profile + 1
    ))
}

fn layer_reports(
    profile: u8,
    layer: u8,
    layer_state: &LayerState,
) -> Result<Vec<[u8; REPORT_SIZE]>, String> {
    if profile >= 3 || layer >= 3 {
        return Err("profile and layer must be 0, 1, or 2".into());
    }
    if layer_state.layer != layer {
        return Err("layer data does not match the requested target layer".into());
    }
    layer_state.validate()?;
    let mut reports = vec![build_key_matrix(profile, layer, &layer_state.key_matrix)?];
    reports.extend(lighting_reports(profile, layer, &layer_state.lighting)?);
    reports.push(build_polling_rate(layer_state.polling_hz, 1, 0)?);
    Ok(reports)
}

fn send_reports<Report: AsRef<[u8]>>(
    reports: &[Report],
    send: &mut impl FnMut(&Report) -> Result<(), String>,
) -> Result<(), String> {
    for report in reports {
        transport::feature_report(report.as_ref())?;
    }
    for (completed, report) in reports.iter().enumerate() {
        send(report).map_err(|error| {
            format!(
                "Report {}/{} failed after {completed} completed transfers; \
                 device state may be partially changed: {error}. No automatic retry.",
                completed + 1,
                reports.len()
            )
        })?;
    }
    Ok(())
}

fn lighting_reports(
    profile: u8,
    layer: u8,
    lighting: &LightingState,
) -> Result<Vec<[u8; REPORT_SIZE]>, String> {
    if profile >= 3 || layer >= 3 {
        return Err("profile and layer must be 0, 1, or 2".into());
    }
    lighting.validate()?;
    let type_pkt = build_led_type(
        profile,
        layer,
        lighting.effect_ui_value,
        lighting.brightness,
        true,
        lighting.input_latency_ms,
    );

    let fx_pkt = build_led_effect(
        lighting.effect_ui_value,
        lighting.speed,
        lighting.brightness,
        lighting.rgb,
        false,
    );
    let mut reports = vec![type_pkt, fx_pkt];

    if let Some(colors) = &lighting.per_key_colors {
        let mut data_buffer = vec![0u8; 800];
        for color in colors {
            let index = color.matrix_index;
            data_buffer[index * 5] = color.visible;
            data_buffer[index * 5 + 1] = index as u8;
            data_buffer[index * 5 + 2] = color.r;
            data_buffer[index * 5 + 3] = color.g;
            data_buffer[index * 5 + 4] = color.b;
        }
        for chunk in 0..4u8 {
            let start = (chunk as usize) * 200;
            let end = start + 200;
            let chunk_pkt = build_led_layout_chunk(
                profile,
                layer,
                chunk,
                &data_buffer[start..end],
                20,
            )?;
            reports.push(chunk_pkt);
        }
    }
    Ok(reports)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::profile::{PerKeyColor, ProfileFile};

    #[test]
    fn interface_selection_handles_missing_unique_duplicate_and_ambiguous_paths() {
        let first = CString::new("first-interface").unwrap();
        let second = CString::new("second-interface").unwrap();
        assert_eq!(unique_interface_path(std::iter::empty::<&CStr>()).unwrap(), None);
        assert_eq!(unique_interface_path([first.as_c_str()].into_iter()).unwrap(), Some(first.clone()));
        assert_eq!(unique_interface_path([first.as_c_str(), first.as_c_str()].into_iter()).unwrap(), Some(first.clone()));
        assert!(unique_interface_path([first.as_c_str(), second.as_c_str()].into_iter()).is_err());
    }

    fn layers() -> [LayerState; 3] {
        let profile: ProfileFile = serde_json::from_str(include_str!(
            "../../assets/factory_profile.gmmkprofile.json"
        ))
        .unwrap();
        profile.profiles[0].layers.clone().try_into().unwrap()
    }

    #[test]
    fn invalid_transport_tail_prevents_the_entire_batch() {
        let valid = build_led_type(0, 0, 0, 100, true, 16);
        let mut invalid = valid;
        invalid[256] = 1;
        let mut writes = 0;
        let result = send_reports(&[valid, invalid], &mut |_| {
            writes += 1;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(writes, 0);
    }

    pub(crate) fn isolated_handle() -> (DeviceHandle, Sender<DeviceEvent>) {
        let (device, events, _commands) = observed_handle();
        (device, events)
    }

    #[test]
    fn every_write_submission_reports_a_closed_command_queue() {
        let (device, _events, commands) = observed_handle();
        drop(commands);
        let layers = layers();
        let results = [
            device.switch_profile(0, 0),
            device.apply_layer(0, 0, layers[0].clone()),
            device.apply_keybindings(0, 0, layers[0].key_matrix.clone()),
            device.apply_lighting(0, 0, layers[0].lighting.clone()),
            device.apply_full_profile(0, layers),
        ];
        for result in results {
            assert!(result.unwrap_err().contains("write was not queued"));
        }
    }

    pub(crate) fn observed_handle() -> (DeviceHandle, Sender<DeviceEvent>, Receiver<DeviceCommand>) {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        (
            DeviceHandle {
                cmd_tx,
                events: evt_rx,
                status: Mutex::new(DeviceStatus::Disconnected),
            },
            evt_tx,
            cmd_rx,
        )
    }

    #[test]
    fn lighting_save_commits_each_windows_sequence_with_exact_sizes_and_delays() {
        for profile in 0..3 {
            for layer in 0..3 {
                for colors in [None, Some(Vec::new()), Some(vec![PerKeyColor {
                    matrix_index: 81, visible: 254, r: 11, g: 22, b: 33,
                }])] {
                    let mut lighting = layers()[0].lighting.clone();
                    lighting.per_key_colors = colors.clone();
                    lighting.input_latency_ms = 8;
                    let mut sent = Vec::new();
                    let message = apply_lighting(profile, layer, &lighting, &mut |report, delay| {
                        sent.push((report.to_vec(), delay));
                        Ok(())
                    }).unwrap();
                    let expected = if colors.is_some() { vec![7, 2, 1, 6, 6, 6, 6, 1] } else { vec![7, 2, 1] };
                    assert_eq!(sent.iter().map(|(report, _)| report[1]).collect::<Vec<_>>(), expected);
                    assert_eq!(sent[0].0[10], 8);
                    for (report, delay) in &sent {
                        assert_eq!(report.len(), if report[1] == 1 { 64 } else { 264 });
                        assert_eq!(*delay, Duration::from_millis(if report[1] == 1 { 550 } else { 100 }));
                        if report[1] != 2 {
                            assert_eq!(&report[..4], &[7, report[1], profile + 1, layer + 1]);
                        }
                        if report[1] == 1 {
                            assert!(report[4..].iter().all(|byte| *byte == 0));
                        }
                    }
                    if let Some(colors) = colors {
                        for page in 0..4 {
                            let report = &sent[page + 3].0;
                            assert_eq!(&report[4..8], &[83, page as u8, 20, 0]);
                            let mut expected_data = vec![0; 256];
                            if page == 2 && !colors.is_empty() {
                                expected_data[5..10].copy_from_slice(&[254, 81, 11, 22, 33]);
                            }
                            assert_eq!(&report[8..], expected_data);
                        }
                    }
                    assert!(message.contains("not read back"));
                    assert!(message.contains("input latency"));
                    assert!(message.contains("activation"));
                }
            }
        }
    }

    #[test]
    fn malformed_final_commit_prevents_all_mixed_size_transfers() {
        let lighting = layers()[0].lighting.clone();
        let mut reports: Vec<Vec<u8>> = lighting_reports(0, 0, &lighting).unwrap()
            .iter().map(|report| report.to_vec()).collect();
        let mut invalid_commit = build_change_profile(0, 0).to_vec();
        invalid_commit.pop();
        reports.push(invalid_commit);
        let mut transfers = 0;
        assert!(send_reports(&reports, &mut |_| {
            transfers += 1;
            Ok(())
        }).is_err());
        assert_eq!(transfers, 0);
    }

    #[test]
    fn custom_lighting_groups_match_all_four_windows_pages_including_side_leds() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!("../../assets/windows-custom-lighting-vectors.json")).unwrap();
        for group in fixture["groups"].as_array().unwrap() {
            for profile in 0..3 {
                for layer in 0..3 {
                    for sample in fixture["samples"].as_array().unwrap() {
                        let rgb: [u8; 3] = serde_json::from_value(sample["rgb"].clone()).unwrap();
                        let visible = if sample["breathing"].as_bool().unwrap() { 254 } else { 255 };
                        let colors: Vec<PerKeyColor> = group["indices"].as_array().unwrap().iter().map(|index| PerKeyColor {
                            matrix_index: index.as_u64().unwrap() as usize, r: rgb[0], g: rgb[1], b: rgb[2], visible,
                        }).collect();
                        let mut data = [0; 800];
                        for color in &colors {
                            let offset = color.matrix_index * 5;
                            data[offset..offset + 5].copy_from_slice(&[visible, color.matrix_index as u8, rgb[0], rgb[1], rgb[2]]);
                        }
                        let mut lighting = layers()[0].lighting.clone();
                        lighting.per_key_colors = Some(colors);
                        let original = serde_json::to_value(&lighting).unwrap();
                        let mut sent = Vec::new();
                        apply_lighting(profile, layer, &lighting, &mut |report, delay| {
                            sent.push((report.to_vec(), delay));
                            Ok(())
                        }).unwrap();
                        assert_eq!(sent.iter().map(|(report, _)| report[1]).collect::<Vec<_>>(), [7, 2, 1, 6, 6, 6, 6, 1]);
                        for page in 0..4 {
                            let mut expected = vec![0; 264];
                            expected[..7].copy_from_slice(&[7, 6, profile + 1, layer + 1, 83, page as u8, 20]);
                            expected[8..208].copy_from_slice(&data[page * 200..(page + 1) * 200]);
                            assert_eq!(sent[page + 3].0, expected, "group {}", group["label"]);
                            assert_eq!(sent[page + 3].1, Duration::from_millis(100));
                        }
                        assert_eq!(serde_json::to_value(&lighting).unwrap(), original);
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_lighting_is_rejected_before_any_transfer() {
        let original = layers()[0].lighting.clone();
        let mut writes = 0;
        for (profile, layer) in [(3, 0), (0, 3), (255, 0), (0, 255)] {
            assert!(apply_lighting(profile, layer, &original, &mut |_, _| {
                writes += 1;
                Ok(())
            }).is_err());
        }
        for mutation in [
            (|lighting: &mut LightingState| lighting.effect_ui_value = 255) as fn(&mut LightingState),
            |lighting| lighting.speed = 101,
            |lighting| lighting.brightness = 101,
            |lighting| lighting.per_key_colors = Some(vec![PerKeyColor { matrix_index: crate::protocol::LED_COUNT, visible: 255, r: 0, g: 0, b: 0 }]),
            |lighting| lighting.per_key_colors = Some(vec![PerKeyColor { matrix_index: 0, visible: 255, r: 0, g: 0, b: 0 }; 2]),
        ] {
            let mut lighting = original.clone();
            mutation(&mut lighting);
            assert!(apply_lighting(0, 0, &lighting, &mut |_, _| {
                writes += 1;
                Ok(())
            }).is_err());
        }
        assert_eq!(writes, 0);
    }

    #[test]
    fn failed_lighting_transfer_stops_before_later_pages_or_commits_without_retry() {
        let mut lighting = layers()[0].lighting.clone();
        lighting.per_key_colors = Some(Vec::new());
        for failure in 0..8 {
            let mut attempted = 0;
            let error = apply_lighting(0, 0, &lighting, &mut |_, _| {
                attempted += 1;
                if attempted == failure + 1 { Err("unplugged".into()) } else { Ok(()) }
            }).unwrap_err();
            assert_eq!(attempted, failure + 1);
            assert!(error.contains(&format!("{failure} completed transfers")));
            assert!(error.contains("partially changed"));
            assert!(error.contains("No automatic retry"));
        }
    }

    #[test]
    fn explicit_empty_custom_lighting_builds_four_zero_pages() {
        let mut lighting = layers()[0].lighting.clone();
        lighting.per_key_colors = Some(Vec::new());
        let reports = lighting_reports(1, 2, &lighting).unwrap();
        let pages: Vec<_> = reports.iter().filter(|report| report[1] == 6).collect();
        assert_eq!(pages.len(), 4);
        for (page, report) in pages.iter().enumerate() {
            assert_eq!(&report[..8], &[7, 6, 2, 3, 83, page as u8, 20, 0]);
            assert!(report[8..].iter().all(|byte| *byte == 0));
        }
        lighting.per_key_colors = None;
        assert_eq!(lighting_reports(1, 2, &lighting).unwrap().len(), 2);
    }

    #[test]
    fn preset_side_light_byte_uses_the_target_capability() {
        let mut lighting = layers()[0].lighting.clone();
        for legacy_toggle in [false, true] {
            lighting.side_light_on = legacy_toggle;
            let reports = lighting_reports(0, 0, &lighting).unwrap();
            assert_eq!(reports[0][7], 0);
            assert_eq!(lighting.side_light_on, legacy_toggle);
        }
    }

    #[test]
    fn activation_uses_the_windows_frame_and_550_ms_delay() {
        let mut sent = Vec::new();
        activate_profile(1, 2, &mut |report, delay| {
            sent.push((report.to_vec(), delay));
            Ok(())
        }).unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0.len(), 64);
        assert_eq!(&sent[0].0[..4], &[7, 1, 2, 3]);
        assert!(sent[0].0[4..].iter().all(|byte| *byte == 0));
        assert_eq!(sent[0].1, Duration::from_millis(550));
    }

    #[test]
    fn activation_rejects_invalid_targets_and_does_not_retry_transport_errors() {
        let mut writes = 0;
        for (profile, layer) in [(3, 0), (0, 3), (255, 0), (0, 255)] {
            assert!(activate_profile(profile, layer, &mut |_report, _delay| {
                writes += 1;
                Ok(())
            }).is_err());
        }
        assert_eq!(writes, 0);
        let error = activate_profile(0, 0, &mut |_report, _delay| {
            writes += 1;
            Err("disconnected".into())
        }).unwrap_err();
        assert_eq!(writes, 1);
        assert!(error.contains("0 completed transfers"));
        assert!(error.contains("No automatic retry"));
    }

    #[test]
    fn keybinding_save_sends_only_the_windows_matrix_report_with_150_ms_delay() {
        let matrix: Vec<u8> = (0..240).collect();
        let mut sent = Vec::new();
        let message = apply_keybindings(2, 1, &matrix, &mut |report, delay| {
            sent.push((report.to_vec(), delay));
            Ok(())
        }).unwrap();
        assert_eq!(sent.len(), 1);
        let (report, delay) = &sent[0];
        assert_eq!(report.len(), 264);
        assert_eq!(&report[..8], &[7, 3, 3, 2, 0, 0, 0, 0]);
        assert_eq!(&report[8..248], matrix.as_slice());
        assert!(report[248..].iter().all(|byte| *byte == 0));
        assert_eq!(*delay, Duration::from_millis(150));
        assert!(message.contains("keybindings only"));
        assert!(message.contains("not read back"));
    }

    #[test]
    fn invalid_keybinding_targets_and_payloads_send_nothing() {
        for (profile, layer, length) in [(3, 0, 240), (255, 0, 240), (0, 3, 240), (0, 255, 240), (0, 0, 239), (0, 0, 241)] {
            let mut writes = 0;
            let result = apply_keybindings(profile, layer, &vec![0; length], &mut |_report, _delay| {
                writes += 1;
                Ok(())
            });
            assert!(result.is_err());
            assert_eq!(writes, 0);
        }
    }

    #[test]
    fn interrupted_keybinding_save_is_not_retried_or_followed_by_other_writes() {
        let mut writes = 0;
        let error = apply_keybindings(0, 0, &[0; 240], &mut |_report, _delay| {
            writes += 1;
            Err("transport failed".into())
        }).unwrap_err();
        assert_eq!(writes, 1);
        assert!(error.contains("0 completed transfers"));
        assert!(error.contains("partially changed"));
        assert!(error.contains("No automatic retry"));
        assert!(error.contains("transport failed"));
    }

    #[test]
    fn firmware_info_is_derived_from_cached_metadata_without_a_hid_device() {
        for (release, expected) in [(None, None), (Some(0), None), (Some(0x123a), None), (Some(0x0123), Some("0123"))] {
            match cached_firmware_info(release) {
                DeviceEvent::Info { fw_version, message } => {
                    assert_eq!(fw_version.as_deref(), expected);
                    assert!(!message.is_empty());
                    if let Some(raw) = release {
                        assert!(message.contains(&format!("{raw:04X}")));
                        assert!(message.contains("No feature-report query is sent"));
                    } else {
                        assert!(message.contains("No query sent"));
                    }
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
    }

    #[test]
    fn invalid_layer_is_rejected_before_any_hardware_transfer() {
        let original = layers()[0].clone();
        let mut invalid = Vec::new();
        let mut layer = original.clone();
        layer.polling_hz = 8000;
        invalid.push(layer);
        let mut layer = original.clone();
        layer.lighting.effect_ui_value = 255;
        invalid.push(layer);
        let mut layer = original.clone();
        layer.lighting.brightness = 101;
        invalid.push(layer);
        let mut layer = original;
        layer.key_matrix.pop();
        invalid.push(layer);

        for layer in invalid {
            let mut sent = Vec::new();
            let result = apply_layer_to_device(0, 0, &layer, &mut |report| {
                sent.push(*report);
                Ok(())
            });
            assert!(result.is_err());
            assert!(sent.is_empty(), "invalid layer caused {} writes", sent.len());
        }
    }

    #[test]
    fn invalid_last_layer_prevents_writes_to_the_entire_slot() {
        let mut layers = layers();
        layers[2].polling_hz = 8000;
        let mut sent = Vec::new();
        let result = apply_full_profile(1, &layers, &mut |report| {
            sent.push(*report);
            Ok(())
        });
        assert!(result.is_err());
        assert!(sent.is_empty());
    }

    #[test]
    fn invalid_or_mismatched_coordinates_never_reach_transport() {
        let layers = layers();
        for (profile, layer) in [(3, 0), (255, 0), (0, 3), (0, 255), (0, 1)] {
            let mut sent = Vec::new();
            let result = apply_layer_to_device(profile, layer, &layers[0], &mut |report| {
                sent.push(*report);
                Ok(())
            });
            assert!(result.is_err(), "accepted coordinates {profile}/{layer}");
            assert!(sent.is_empty());
        }
        let mut duplicate = layers.clone();
        duplicate[2].layer = 1;
        let mut sent = Vec::new();
        assert!(apply_full_profile(0, &duplicate, &mut |report| {
            sent.push(*report);
            Ok(())
        }).is_err());
        assert!(sent.is_empty());
    }

    #[test]
    fn layer_apply_preserves_raw_bindings_without_altgr_or_slot_switch_writes() {
        let mut layer = layers()[1].clone();
        layer.key_matrix[60] = 251;
        layer.key_matrix[61] = 193;
        layer.polling_hz = 500;
        let mut sent = Vec::new();
        let result = apply_layer_to_device(2, 1, &layer, &mut |report| {
            sent.push(*report);
            Ok(())
        });
        assert!(result.is_ok());
        assert_eq!(sent.iter().map(|report| report[1]).collect::<Vec<_>>(), [3, 7, 2, 8]);
        assert_eq!(&sent[0][..8], &[7, 3, 3, 2, 0, 0, 0, 0]);
        assert_eq!(&sent[0][8..248], layer.key_matrix.as_slice());
        assert_eq!(&sent[3][8..11], &[1, 1, 0]);
    }

    #[test]
    fn per_key_pages_keep_windows_fixed_brightness_and_color_bytes() {
        let mut layer = layers()[0].clone();
        layer.lighting.brightness = 25;
        layer.lighting.per_key_colors = Some(vec![PerKeyColor {
            matrix_index: 81,
            visible: 254,
            r: 11,
            g: 22,
            b: 33,
        }]);
        let mut sent = Vec::new();
        apply_layer_to_device(1, 0, &layer, &mut |report| {
            sent.push(*report);
            Ok(())
        }).unwrap();
        let pages: Vec<_> = sent.iter().filter(|report| report[1] == 6).collect();
        assert_eq!(pages.len(), 4);
        for (page, report) in pages.iter().enumerate() {
            assert_eq!(&report[..8], &[7, 6, 2, 1, 83, page as u8, 20, 0]);
        }
        assert_eq!(&pages[2][13..18], &[254, 81, 11, 22, 33]);
    }

    #[test]
    fn interrupted_slot_write_stops_without_retry_and_reports_partial_state() {
        let mut attempted = 0;
        let result = apply_full_profile(0, &layers(), &mut |_report| {
            attempted += 1;
            if attempted == 6 {
                Err("incomplete send".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(attempted, 6);
        let error = result.unwrap_err();
        assert!(error.contains("5 completed"), "{error}");
        assert!(error.contains("partially changed"), "{error}");
        assert!(error.contains("incomplete send"), "{error}");
    }
}
