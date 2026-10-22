mod config;
mod core;
mod engine;
mod system;

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use config::Config;
use core::Metadata;
use engine::{Details, Engine, Snapshot};
use system::Strings;

pub fn log(message: &str) {
    eprintln!("{message}");
    let dir = config::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("tormin.log")) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let _ = writeln!(f, "{secs} {message}");
    }
}

/// .torrent files and magnet links Tormin was asked to open.
#[derive(Serialize, Clone, Default)]
struct OpenRequest {
    files: Vec<String>,
    magnets: Vec<String>,
}

impl OpenRequest {
    fn is_empty(&self) -> bool {
        self.files.is_empty() && self.magnets.is_empty()
    }
}

fn parse_args(args: &[String]) -> (OpenRequest, bool) {
    let mut req = OpenRequest::default();
    let mut minimized = false;
    for arg in args.iter().skip(1) {
        let arg = arg.trim().trim_matches('"');
        if arg.len() >= 7 && arg[..7].eq_ignore_ascii_case("magnet:") {
            req.magnets.push(arg.to_string());
        } else if arg.to_ascii_lowercase().ends_with(".torrent") {
            req.files.push(arg.to_string());
        } else if arg == "--minimized" {
            minimized = true;
        }
    }
    (req, minimized)
}

struct Tray {
    icon: TrayIcon,
    show: MenuItem<tauri::Wry>,
    resume_all: MenuItem<tauri::Wry>,
    pause_all: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

#[derive(Default)]
struct AppState {
    engine: Mutex<Option<Engine>>,
    engine_error: Mutex<Option<String>>,
    pending_open: Mutex<Vec<OpenRequest>>,
    tray: Mutex<Option<Tray>>,
    quitting: AtomicBool,
    /// The window was moved or resized: the poll loop saves its geometry
    geometry_changed: AtomicBool,
}

impl AppState {
    fn with<T>(&self, f: impl FnOnce(&mut Engine) -> T) -> Result<T, String> {
        let mut guard = self.engine.lock().unwrap();
        match guard.as_mut() {
            Some(engine) => Ok(f(engine)),
            None => Err(self.engine_error.lock().unwrap().clone().unwrap_or_else(|| "libtorrent is not running".into())),
        }
    }
}

// ------------------------------------------------------------------ settings

/// Settings the UI may change (everything but the saved torrent list).
#[derive(Serialize, Deserialize, Clone)]
struct UiConfig {
    download_dir: String,
    language: String,
    theme: String,
    accent: String,
    compact_mode: bool,
    compact_window_size: bool,
    progress_style: u8,
    monochrome: bool,
    shutdown_complete: bool,
    start_with_windows: bool,
    start_minimized: bool,
}

impl UiConfig {
    fn from(c: &Config) -> UiConfig {
        UiConfig {
            download_dir: c.download_dir(),
            language: c.language.clone(),
            theme: c.theme.clone(),
            accent: c.accent.clone(),
            compact_mode: c.compact_mode,
            compact_window_size: c.compact_window_size,
            progress_style: c.progress_style,
            monochrome: c.monochrome,
            shutdown_complete: c.shutdown_complete,
            start_with_windows: c.start_with_windows,
            start_minimized: c.start_minimized,
        }
    }

    fn apply(self, c: &mut Config) {
        c.download_dir = Some(self.download_dir);
        c.language = self.language;
        c.theme = self.theme;
        c.accent = self.accent;
        c.compact_mode = self.compact_mode;
        c.compact_window_size = self.compact_window_size;
        c.progress_style = self.progress_style.clamp(1, 3);
        c.monochrome = self.monochrome;
        c.shutdown_complete = self.shutdown_complete;
        c.start_with_windows = self.start_with_windows;
        c.start_minimized = self.start_minimized;
    }
}

#[derive(Serialize)]
struct InitialState {
    config: UiConfig,
    snapshot: Option<Snapshot>,
    version: String,
    libtorrent: String,
    platform: &'static str,
    error: Option<String>,
}

// ------------------------------------------------------------------ commands

#[tauri::command]
fn get_state(app: AppHandle, state: State<AppState>) -> InitialState {
    let engine = state.engine.lock().unwrap();
    let config = engine.as_ref().map(|e| e.config.clone()).unwrap_or_else(Config::load);
    InitialState {
        config: UiConfig::from(&config),
        snapshot: engine.as_ref().map(|e| e.snapshot()),
        version: app.package_info().version.to_string(),
        libtorrent: engine.as_ref().map(|e| e.version()).unwrap_or_default(),
        platform: if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" },
        error: state.engine_error.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn take_pending_open(state: State<AppState>) -> Vec<OpenRequest> {
    std::mem::take(&mut *state.pending_open.lock().unwrap())
}

#[tauri::command]
fn read_torrent_file(state: State<AppState>, path: String) -> Result<Metadata, String> {
    state.with(|e| e.read_torrent_file(&path))?
}

#[tauri::command]
fn add_torrent_file(state: State<AppState>, path: String, priorities: Option<Vec<u8>>) -> Result<u32, String> {
    state.with(|e| e.add_torrent_file(&path, priorities, false))?
}

#[tauri::command]
fn add_magnet(state: State<AppState>, uri: String) -> Result<u32, String> {
    state.with(|e| e.add_magnet(&uri, true, false))?
}

#[tauri::command]
fn pause(state: State<AppState>, ids: Vec<u32>) -> Result<(), String> {
    state.with(|e| e.pause(&ids))
}

#[tauri::command]
fn resume(state: State<AppState>, ids: Vec<u32>) -> Result<(), String> {
    state.with(|e| e.resume(&ids))
}

#[tauri::command]
fn remove(state: State<AppState>, ids: Vec<u32>, delete_files: bool) -> Result<(), String> {
    state.with(|e| e.remove(&ids, delete_files))
}

#[tauri::command]
fn torrent_details(state: State<AppState>, id: u32) -> Result<Details, String> {
    state.with(|e| e.details(id))
}

#[tauri::command]
fn apply_properties(state: State<AppState>, id: u32, download_limit_kb: i32, priorities: Option<Vec<u8>>) -> Result<(), String> {
    state.with(|e| e.apply_properties(id, download_limit_kb, priorities))
}

#[tauri::command]
fn complete_file_selection(state: State<AppState>, id: u32, priorities: Option<Vec<u8>>) -> Result<(), String> {
    state.with(|e| e.complete_file_selection(id, priorities))
}

#[tauri::command]
fn open_folder(app: AppHandle, state: State<AppState>, id: u32) -> Result<(), String> {
    let (dir, name) = state.with(|e| (e.save_path(id), e.name(id)))?;
    // Prefer the torrent's own folder when it has one
    let own = PathBuf::from(&dir).join(&name);
    let target = if !name.is_empty() && own.is_dir() { own } else { PathBuf::from(dir) };
    app.opener().open_path(target.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_config(app: AppHandle, state: State<AppState>, config: UiConfig) -> Result<(), String> {
    let language = config.language.clone();
    let compact_window = config.compact_window_size;
    let autostart = config.start_with_windows;
    let compact_changed = state.with(|e| {
        let changed = e.config.compact_window_size != compact_window;
        config.apply(&mut e.config);
        if changed {
            // The preset wins over the remembered size when the user switches it
            e.config.window_width = None;
            e.config.window_height = None;
        }
        e.config.save();
        changed
    })?;

    let launcher = app.autolaunch();
    if launcher.is_enabled().unwrap_or(false) != autostart {
        let result = if autostart { launcher.enable() } else { launcher.disable() };
        if let Err(err) = result {
            log(&format!("Autostart failed: {err}"));
        }
    }
    update_tray_texts(&app, &language);
    if compact_changed {
        apply_window_size(&app, compact_window);
    }
    Ok(())
}

#[tauri::command]
fn associate_files() -> Result<(), String> {
    system::associate_files()
}

#[tauri::command]
fn quit(app: AppHandle) {
    quit_app(&app);
}

// ------------------------------------------------------------------ app logic

fn core_library(app: &AppHandle) -> PathBuf {
    let name = if cfg!(windows) { "TorminCore.dll" } else { "libTorminCore.so" };
    let mut candidates = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        candidates.push(dir.join(name));
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        candidates.push(dir.join(name));
        candidates.push(dir.join("../lib/tormin").join(name));
    }
    // Development: straight from the CMake build directory
    let preset = if cfg!(windows) { "x64-release" } else { "linux-release" };
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../core/build").join(preset).join(name));
    candidates.iter().find(|p| p.exists()).cloned().unwrap_or_else(|| PathBuf::from(name))
}

fn show_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn toggle_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) {
            hide_to_tray(app, &w);
        } else {
            show_window(app);
        }
    }
}

/// Closing the window keeps Tormin running in the tray, like the other versions.
fn hide_to_tray(app: &AppHandle, window: &tauri::WebviewWindow) {
    let geometry = window_geometry(app);
    let _ = app.state::<AppState>().with(|e| {
        if let Some(g) = geometry {
            g.apply(&mut e.config);
        }
        e.save_session(false);
        e.config.save();
    });
    let _ = window.hide();
}

const MIN_HEIGHT: f64 = 420.0;

/// Each mode has a fixed width (outer frame); only the height can be changed.
fn mode_width(compact: bool) -> f64 {
    if compact { 320.0 } else { 650.0 }
}

fn lock_width(w: &tauri::WebviewWindow, compact: bool) {
    let width = mode_width(compact);
    let _ = w.set_min_size(Some(LogicalSize::new(width, MIN_HEIGHT)));
    let _ = w.set_max_size(Some(LogicalSize::new(width, 100_000.0)));
}

/// Logical size of the invisible resize borders around the undecorated window.
/// The size limits apply to the outer frame, so the inner size is the limit minus these.
fn borders(w: &tauri::WebviewWindow) -> (f64, f64) {
    match (w.outer_size(), w.inner_size(), w.scale_factor()) {
        (Ok(o), Ok(i), Ok(s)) => (o.width.saturating_sub(i.width) as f64 / s, o.height.saturating_sub(i.height) as f64 / s),
        _ => (0.0, 0.0),
    }
}

/// Inner size of a preset: compact is the smallest window (the default look); the large one matches the other versions.
fn window_size(w: &tauri::WebviewWindow, compact: bool) -> (f64, f64) {
    let (bw, bh) = borders(w);
    (mode_width(compact) - bw, if compact { MIN_HEIGHT - bh } else { 650.0 })
}

fn apply_window_size(app: &AppHandle, compact: bool) {
    if let Some(w) = app.get_webview_window("main") {
        // Limits first: the old width limit would clamp the new size
        lock_width(&w, compact);
        let (width, height) = window_size(&w, compact);
        let _ = w.set_size(LogicalSize::new(width, height));
        place_window(app, &w);
    }
}

/// Keeps the size the user gave the window for the next start.
fn remember_window_geometry(app: &AppHandle, config: &mut Config) {
    if let Some(size) = window_geometry(app) {
        size.apply(config);
    }
}

/// Inner size in logical pixels.
struct Geometry {
    width: f64,
    height: f64,
}

impl Geometry {
    fn apply(&self, config: &mut Config) {
        config.window_width = Some(self.width);
        config.window_height = Some(self.height);
    }
}

/// Reads the window from the OS: call it without holding the engine lock.
fn window_geometry(app: &AppHandle) -> Option<Geometry> {
    let w = app.get_webview_window("main")?;
    // A hidden or minimized window reports a meaningless size
    if w.is_minimized().unwrap_or(false) || !w.is_visible().unwrap_or(false) {
        return None;
    }
    let logical = w.inner_size().ok()?.to_logical::<f64>(w.scale_factor().ok()?);
    Some(Geometry { width: logical.width.round(), height: logical.height.round() })
}

/// Always the bottom-right corner of the primary monitor's work area (above the taskbar).
fn place_window(app: &AppHandle, window: &tauri::WebviewWindow) {
    let (Ok(outer), Ok(inner), Ok(Some(monitor))) = (window.outer_size(), window.inner_size(), app.primary_monitor()) else {
        return;
    };
    let area = monitor.work_area();
    let margin = (12.0 * monitor.scale_factor()) as i32;
    // The margin is measured from the visible edges: the invisible resize borders
    // are split between left and right, and lie below the window at the bottom
    let border_right = (outer.width.saturating_sub(inner.width) / 2) as i32;
    let border_bottom = outer.height.saturating_sub(inner.height) as i32;
    let x = area.position.x + area.size.width as i32 - outer.width as i32 + border_right - margin;
    let y = area.position.y + area.size.height as i32 - outer.height as i32 + border_bottom - margin;
    let _ = window.set_position(tauri::PhysicalPosition::new(x.max(area.position.x), y.max(area.position.y)));
}

/// The main window is created here rather than in tauri.conf.json so it starts at the configured size.
fn create_window(app: &AppHandle, config: &Config) -> tauri::Result<()> {
    let builder = tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::default())
        .title("Tormin")
        .inner_size(mode_width(true), MIN_HEIGHT)
        .min_inner_size(mode_width(true), MIN_HEIGHT)
        .maximizable(false)
        .decorations(false)
        .visible(false);
    // Windows 11: Mica backdrop behind the semi-transparent UI
    #[cfg(windows)]
    let builder = builder.transparent(true).effects(
        tauri::window::EffectsBuilder::new().effect(tauri::window::Effect::Mica).build(),
    );
    let window = builder.build()?;
    // The borders (and so the real inner size) are known only once the window exists
    lock_width(&window, config.compact_window_size);
    let (width, preset_height) = window_size(&window, config.compact_window_size);
    let min_height = window_size(&window, true).1;
    let height = config.window_height.map_or(preset_height, |h| h.max(min_height));
    let _ = window.set_size(LogicalSize::new(width, height));
    place_window(app, &window);
    Ok(())
}

fn quit_app(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(mut engine) = state.engine.lock().unwrap().take() {
        remember_window_geometry(app, &mut engine.config);
        engine.save_session_sync(Duration::from_secs(5));
    }
    app.exit(0);
}

fn handle_open_request(app: &AppHandle, req: OpenRequest) {
    show_window(app);
    if req.is_empty() {
        return;
    }
    // Queued, not sent: the UI may not be loaded yet. It takes the queue on start and on this signal.
    app.state::<AppState>().pending_open.lock().unwrap().push(req);
    let _ = app.emit("open-pending", ());
}

fn update_tray_texts(app: &AppHandle, language: &str) {
    let strings = Strings::new(language);
    if let Some(tray) = app.state::<AppState>().tray.lock().unwrap().as_ref() {
        let _ = tray.show.set_text(strings.t("show"));
        let _ = tray.resume_all.set_text(strings.t("resume_all"));
        let _ = tray.pause_all.set_text(strings.t("pause_all"));
        let _ = tray.quit.set_text(strings.t("quit"));
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<Tray> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let resume_all = MenuItem::with_id(app, "resume_all", "Resume All", true, None::<&str>)?;
    let pause_all = MenuItem::with_id(app, "pause_all", "Pause All", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &sep1, &resume_all, &pause_all, &sep2, &quit])?;

    let icon = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundle icon"))
        .tooltip("Tormin")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id().as_ref() {
                "show" => show_window(app),
                "resume_all" => {
                    let _ = state.with(|e| e.resume(&e.all_ids()));
                }
                "pause_all" => {
                    let _ = state.with(|e| e.pause(&e.all_ids()));
                }
                "quit" => quit_app(app),
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(Tray { icon, show, resume_all, pause_all, quit })
}

fn format_speed(bytes: i64) -> String {
    match bytes {
        b if b > 1024 * 1024 => format!("{:.1} MB/s", b as f64 / 1024.0 / 1024.0),
        b if b > 1024 => format!("{:.1} KB/s", b as f64 / 1024.0),
        b => format!("{b} B/s"),
    }
}

/// Once a second: libtorrent events, status snapshot for the UI, tray tooltip.
fn poll_loop(app: AppHandle) {
    let mut last_tip = String::new();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let state = app.state::<AppState>();
        if state.quitting.load(Ordering::SeqCst) {
            return;
        }
        // Saved right away, not only on close: a killed process (Ctrl+C in dev mode) never closes
        let geometry = if state.geometry_changed.swap(false, Ordering::SeqCst) { window_geometry(&app) } else { None };
        let (snapshot, outcome) = {
            let mut guard = state.engine.lock().unwrap();
            let Some(engine) = guard.as_mut() else { continue };
            if let Some(g) = geometry {
                g.apply(&mut engine.config);
                engine.config.save();
            }
            let outcome = engine.tick();
            (engine.snapshot(), outcome)
        };
        let _ = app.emit("torrents", &snapshot);

        let mut tip = String::from("Tormin");
        if snapshot.download_rate > 0 {
            tip += &format!("\n▼ {}", format_speed(snapshot.download_rate));
        }
        if snapshot.upload_rate > 0 {
            tip += &format!("\n▲ {}", format_speed(snapshot.upload_rate));
        }
        if tip != last_tip {
            if let Some(tray) = state.tray.lock().unwrap().as_ref() {
                let _ = tray.icon.set_tooltip(Some(&tip));
            }
            last_tip = tip;
        }

        for (id, metadata) in outcome.select_files {
            show_window(&app);
            let _ = app.emit("select-files", serde_json::json!({ "id": id, "metadata": metadata }));
        }
        if !outcome.finished.is_empty() {
            let language = state.with(|e| e.config.language.clone()).unwrap_or_default();
            let title = Strings::new(&language).t("download_finished");
            for name in outcome.finished {
                let _ = app.notification().builder().title(&title).body(&name).show();
            }
        }
        for message in outcome.errors {
            let _ = app.emit("error", message);
        }
        if outcome.all_completed {
            log("All torrents completed, shutting down...");
            let _ = state.with(|e| e.save_session_sync(Duration::from_secs(5)));
            system::shutdown();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch hands its arguments to the running instance and exits
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let (req, _) = parse_args(&argv);
            handle_open_request(app, req);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            let config = Config::load();
            let language = config.language.clone();
            let start_minimized = config.start_minimized;
            create_window(&handle, &config)?;

            match Engine::start(&core_library(&handle), config) {
                Ok(engine) => *state.engine.lock().unwrap() = Some(engine),
                Err(e) => {
                    log(&format!("Engine failed: {e}"));
                    *state.engine_error.lock().unwrap() = Some(e);
                }
            }

            *state.tray.lock().unwrap() = Some(build_tray(&handle)?);
            update_tray_texts(&handle, &language);

            let args: Vec<String> = std::env::args().collect();
            let (req, minimized) = parse_args(&args);
            let has_files = !req.is_empty();
            if has_files {
                state.pending_open.lock().unwrap().push(req);
            }
            if !(minimized || start_minimized) || has_files {
                show_window(&handle);
            }

            std::thread::spawn(move || poll_loop(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    if !app.state::<AppState>().quitting.load(Ordering::SeqCst) {
                        api.prevent_close();
                        if let Some(w) = app.get_webview_window(window.label()) {
                            hide_to_tray(app, &w);
                        }
                    }
                }
                WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                    app.state::<AppState>().geometry_changed.store(true, Ordering::SeqCst);
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            take_pending_open,
            read_torrent_file,
            add_torrent_file,
            add_magnet,
            pause,
            resume,
            remove,
            torrent_details,
            apply_properties,
            complete_file_selection,
            open_folder,
            set_config,
            associate_files,
            quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
