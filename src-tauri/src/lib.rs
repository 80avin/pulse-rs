use std::sync::{Arc, Mutex, OnceLock};

use pulse_core::{PulseCore, config::PulseConfig};
#[cfg(target_os = "android")]
use tauri::Emitter;
use tauri::Manager;

mod commands;
mod models;

/// Type alias for the reloadable log-filter handle stored in AppState.
type LogFilterHandle =
    tracing_subscriber::reload::Handle<tracing_subscriber::EnvFilter, tracing_subscriber::Registry>;

/// Map a verbose flag to a tracing directive string.
pub(crate) fn log_directive(verbose: bool) -> &'static str {
    if verbose {
        "pulse_core=debug,pulse_rs_lib=debug,pulse_frontend=debug,debug"
    } else {
        "pulse_core=info,pulse_rs_lib=info,warn"
    }
}

/// Read `verboseLogging` from the persisted settings JSON without initialising
/// PulseCore — called before tracing is set up so the first log event uses
/// the correct level.
fn read_verbose_setting(data_dir: &std::path::Path) -> bool {
    std::fs::read_to_string(data_dir.join("tauri_settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["verboseLogging"].as_bool())
        .unwrap_or(false)
}

/// Set up the tracing subscriber for the lifetime of the app.
///
/// Returns:
/// - `WorkerGuard` — must be kept alive; dropping it shuts down the file-flush thread.
/// - `LogFilterHandle` — call `.modify()` on it to change the log level at runtime.
///
/// Both platforms write to a rolling daily file in `{data_dir}/logs/` (7-day retention).
/// Desktop also echoes to stderr for development convenience.
fn init_tracing(
    data_dir: &std::path::Path,
    verbose: bool,
) -> (tracing_appender::non_blocking::WorkerGuard, LogFilterHandle) {
    use tracing_subscriber::{
        EnvFilter, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt,
    };

    let directive = log_directive(verbose);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(directive));
    let (filter_layer, filter_handle) =
        reload::Layer::<EnvFilter, tracing_subscriber::Registry>::new(filter);

    let log_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("pulse.log")
        .max_log_files(7)
        .build(&log_dir)
        .unwrap_or_else(|_| tracing_appender::rolling::daily(&log_dir, "pulse.log"));
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Desktop: file + stderr. Android: file only (stderr goes nowhere useful there).
    #[cfg(not(target_os = "android"))]
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(fmt::layer().with_writer(std::io::stderr))
        .init();

    #[cfg(target_os = "android")]
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    (guard, filter_handle)
}

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
pub(crate) static PENDING_SHARE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Process start instant — used to gate cold-start share buffering (below).
static BOOT_TIME: OnceLock<std::time::Instant> = OnceLock::new();
#[cfg(target_os = "android")]
static ANDROID_VM: OnceLock<jni::JavaVM> = OnceLock::new();

/// Represents the initialization state of PulseCore so commands can
/// distinguish "still loading" from "failed permanently".
#[derive(Clone)]
enum InitState {
    Pending,
    Ready(Arc<PulseCore>),
    Failed(String),
}

pub struct AppState {
    /// Tracks PulseCore initialization. Commands await this before proceeding.
    init_rx: tokio::sync::watch::Receiver<InitState>,
    /// Available immediately — before PulseCore is ready. Used by diagnostic commands.
    pub data_dir: std::path::PathBuf,
    /// Live handle to change the tracing filter without restarting.
    pub log_filter: LogFilterHandle,
    /// Keeps the background log-flush thread alive for the app lifetime.
    _log_guard: tracing_appender::non_blocking::WorkerGuard,
}

impl AppState {
    /// Wait for PulseCore to finish initializing and return a clone of the Arc.
    /// Returns almost instantly on warm calls (watch channel is already set).
    /// Returns `Err` if initialization failed
    pub async fn core(&self) -> Result<Arc<PulseCore>, String> {
        let mut rx = self.init_rx.clone();
        loop {
            match rx.borrow().clone() {
                InitState::Ready(c) => return Ok(c),
                InitState::Failed(e) => return Err(e),
                InitState::Pending => {}
            }
            if rx.changed().await.is_err() {
                let msg = "PulseCore init task exited without completing".to_string();
                tracing::error!("{msg}");
                return Err(msg);
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = PENDING_SHARE.set(Mutex::new(None));
    let _ = BOOT_TIME.set(std::time::Instant::now());

    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_sharesheet::init());

    builder
        .setup(move |app| {
            let t_setup = std::time::Instant::now();

            // Persist the AppHandle so the JNI bridge can emit events
            let _ = APP_HANDLE.set(app.handle().clone());

            #[cfg(target_os = "android")]
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| pulse_core::config::platform_data_dir());
            #[cfg(not(target_os = "android"))]
            let data_dir = pulse_core::config::platform_data_dir();

            std::fs::create_dir_all(&data_dir)?;
            let verbose = read_verbose_setting(&data_dir);
            let (log_guard, log_filter) = init_tracing(&data_dir, verbose);

            tracing::info!(
                elapsed_ms = t_setup.elapsed().as_millis(),
                "coldstart: setup: data_dir ready"
            );

            tracing::info!(
                elapsed_ms = t_setup.elapsed().as_millis(),
                "coldstart: setup: data_dir ready"
            );

            // Build config before moving data_dir into AppState.
            let config = PulseConfig::default_config().with_data_dir(data_dir.clone());

            // Manage AppState immediately so Tauri can start dispatching queued IPC.
            // Commands await `state.core()` which blocks on init_rx until PulseCore is ready.
            let (init_tx, init_rx) = tokio::sync::watch::channel(InitState::Pending);
            app.manage(AppState {
                init_rx,
                data_dir,
                log_filter,
                _log_guard: log_guard,
            });
            tracing::info!(
                elapsed_ms = t_setup.elapsed().as_millis(),
                "coldstart: setup: AppState managed, IPC ready"
            );

            // Init PulseCore on a dedicated thread using Handle::block_on.
            // PulseCore::init's future is not Send (sqlx HRTB limitation), so we
            // can't use tauri::async_runtime::spawn which requires Send + 'static.
            // Handle::block_on has no Send requirement and keeps all tokio tasks
            // (db writer, tagger) on the shared Tauri runtime.
            let rt_handle = tauri::async_runtime::handle();
            std::thread::Builder::new()
                .name("pulse-core-init".into())
                .spawn(move || {
                    rt_handle.block_on(async move {
                        match PulseCore::init(config).await {
                            Ok(core) => {
                                let core = Arc::new(core);
                                let core_bg = Arc::clone(&core);
                                tauri::async_runtime::spawn(
                                    async move { core_bg.start_sync().await },
                                );
                                let _ = init_tx.send(InitState::Ready(core));
                                tracing::info!("coldstart: PulseCore ready, commands unblocked");
                            }
                            Err(e) => {
                                let err_msg = format!("PulseCore init failed: {e}");
                                tracing::error!("{err_msg}");
                                let _ = init_tx.send(InitState::Failed(err_msg));
                            }
                        }
                    });
                })
                .expect("failed to spawn pulse-core-init thread");

            // Drain any share URL that arrived before the WebView was ready (cold-start)
            #[cfg(target_os = "android")]
            {
                if let Some(url) = PENDING_SHARE
                    .get()
                    .and_then(|m| m.lock().ok())
                    .and_then(|mut g| g.take())
                {
                    let h = app.handle().clone();
                    tauri::async_runtime::spawn(async move {
                        // Give WebView time to register its event listener
                        tokio::time::sleep(std::time::Duration::from_millis(800)).await;
                        let _ = h.emit(
                            "share://incoming-url",
                            crate::models::IncomingShareEvent { url },
                        );
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sources,
            commands::add_source,
            commands::delete_source,
            commands::update_source,
            commands::get_items_page,
            commands::mark_items_read,
            commands::mark_source_read,
            commands::toggle_saved,
            commands::set_item_note,
            commands::hide_item,
            commands::get_user_tags,
            commands::set_user_tags,
            commands::get_groups,
            commands::add_group,
            commands::rename_group,
            commands::delete_group,
            commands::sync_source,
            commands::sync_all,
            commands::get_settings,
            commands::save_settings,
            commands::get_db_stats,
            commands::search_items,
            // AI management
                                                                                    commands::get_tag_stats,
            // Share intent
            commands::detect_feed,
            commands::get_pending_share,
            // Frontend log bridge
            commands::log_from_frontend,
            // Diagnostics
            commands::set_log_level,
            commands::get_log_content,
            commands::get_log_path,
            commands::open_logs_folder,
            commands::share_log_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// JNI function called from ShareBridge.kt during MainActivity.onCreate.
/// Receives the Android Application context so the platform TLS verifier
/// can access the system trust store on Android.
/// Also caches the JavaVM so background commands can call Kotlin methods.
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_avinthakur080_pulse_1rs_ShareBridge_init<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    context: jni::objects::JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if let Ok(vm) = env.get_java_vm() {
            let _ = ANDROID_VM.set(vm);
        }
        if let Err(e) = rustls_platform_verifier::android::init_with_env(env, context) {
            tracing::error!("rustls-platform-verifier android init failed: {e}");
        }
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

/// JNI function called from ShareBridge.kt when Android receives a share/view intent.
/// The symbol name encodes the Kotlin package+class: com.avinthakur080.pulse_rs.ShareBridge.onShareUrl
/// Note: underscores in package names are encoded as _1 in JNI.
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_avinthakur080_pulse_1rs_ShareBridge_onShareUrl<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    url: jni::objects::JString<'local>,
) {
    let url = env
        .with_env(|env| -> jni::errors::Result<_> { Ok(String::from(url.mutf8_chars(env)?)) })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    if url.is_empty() {
        return;
    }

    // Buffer the URL for recovery via get_pending_share, but ONLY during the
    // startup window when the WebView listener may not be live yet. Buffering
    // mid-session would leave a stale URL that pops an old ShareSheet on the
    // next cold start.
    let within_boot_window = BOOT_TIME
        .get()
        .map_or(true, |t| t.elapsed() < std::time::Duration::from_secs(10));
    if within_boot_window {
        if let Some(pending) = PENDING_SHARE.get() {
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(url.clone());
            }
        }
    }

    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit(
            "share://incoming-url",
            crate::models::IncomingShareEvent { url },
        );
    }
}
