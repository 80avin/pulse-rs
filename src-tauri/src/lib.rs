use std::sync::{Arc, Mutex, OnceLock};

use pulse_core::{PulseCore, config::PulseConfig};
#[cfg(target_os = "android")]
use tauri::Emitter;
use tauri::Manager;

mod commands;
mod models;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
static PENDING_SHARE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub struct AppState {
    pub core: Arc<PulseCore>,
    pub pending_share: Arc<Mutex<Option<String>>>,
}

// FastText model embedded at compile time (9.6 MB + tiny thresholds file).
// Extracted to data_dir on first run; re-extracted on version bump.
const BUNDLED_FASTTEXT_PFTM: &[u8] = include_bytes!("../bundled/fasttext-v2/fasttext.pftm");
const BUNDLED_FASTTEXT_THRESHOLDS: &[u8] =
    include_bytes!("../bundled/fasttext-v2/fasttext_thresholds.json");
// Bump this string whenever the bundled fasttext model or thresholds change.
// Users will get the updated model extracted on the next app launch.
const BUNDLED_FASTTEXT_VERSION: &str = "v2-20250519b";

// MiniLM MLP head embedded (~208 KB). The ONNX backbone (87 MB) is downloaded
// separately from HuggingFace; these small files accompany it.
const BUNDLED_MINIML_MLP: &[u8] = include_bytes!("../bundled/minilm/mlp_head.pmlp");
const BUNDLED_MINIML_THRESHOLDS: &[u8] = include_bytes!("../bundled/minilm/miniml_thresholds.json");

/// Extract bundled model bytes to data_dir.
/// FastText is re-extracted whenever BUNDLED_FASTTEXT_VERSION changes so that
/// app updates deliver new vocabulary and thresholds to existing installations.
/// MiniLM supporting files are always overwritten so MLP head improvements land
/// automatically without requiring the user to re-download the ONNX backbone.
fn extract_bundled_models(data_dir: &std::path::Path) {
    // FastText — fully bundled, always available without any download.
    let ft_dir = data_dir.join("models").join("fasttext-v2");
    let version_file = ft_dir.join("version");
    let installed_version = std::fs::read_to_string(&version_file).unwrap_or_default();
    if installed_version.trim() != BUNDLED_FASTTEXT_VERSION {
        if std::fs::create_dir_all(&ft_dir).is_ok() {
            let _ = std::fs::write(ft_dir.join("fasttext.pftm"), BUNDLED_FASTTEXT_PFTM);
            let _ = std::fs::write(
                ft_dir.join("fasttext_thresholds.json"),
                BUNDLED_FASTTEXT_THRESHOLDS,
            );
            let _ = std::fs::write(&version_file, BUNDLED_FASTTEXT_VERSION);
        }
    }
    // Write the active-model pointer if not already configured.
    let ft_ptr = data_dir.join("active_fasttext_model");
    if !ft_ptr.exists() {
        let _ = std::fs::write(&ft_ptr, "fasttext-v2");
    }

    // MiniLM MLP head — bundled; model.onnx must be downloaded by the user.
    // Always written so an app update delivers an improved MLP head without
    // requiring the user to re-download the ONNX backbone.
    let ml_dir = data_dir.join("models").join("minilm");
    if std::fs::create_dir_all(&ml_dir).is_ok() {
        let _ = std::fs::write(ml_dir.join("mlp_head.pmlp"), BUNDLED_MINIML_MLP);
        let _ = std::fs::write(
            ml_dir.join("miniml_thresholds.json"),
            BUNDLED_MINIML_THRESHOLDS,
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = PENDING_SHARE.set(Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Persist the AppHandle so the JNI bridge can emit events
            let _ = APP_HANDLE.set(app.handle().clone());

            // On Android use Tauri's app_data_dir() so the path is keyed by the
            // package ID and survives updates. On desktop keep platform_data_dir()
            // so the Tauri app and the CLI share the same database.
            #[cfg(target_os = "android")]
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| pulse_core::config::platform_data_dir());
            #[cfg(not(target_os = "android"))]
            let data_dir = pulse_core::config::platform_data_dir();

            std::fs::create_dir_all(&data_dir)?;

            // Extract FastText + MiniLM MLP head to data_dir on first run.
            extract_bundled_models(&data_dir);

            let config = PulseConfig::default_config().with_data_dir(data_dir);

            let core = tauri::async_runtime::block_on(PulseCore::init(config))
                .map_err(|e| format!("PulseCore init failed: {e}"))?;

            let core = Arc::new(core);
            let core_bg = Arc::clone(&core);
            tauri::async_runtime::spawn(async move { core_bg.start_sync().await });

            let pending_share = Arc::new(Mutex::new(None));
            app.manage(AppState {
                core,
                pending_share,
            });

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
            commands::hide_item,
            commands::get_groups,
            commands::add_group,
            commands::rename_group,
            commands::delete_group,
            commands::sync_source,
            commands::sync_all,
            commands::get_settings,
            commands::save_settings,
            commands::get_db_stats,
            commands::clear_items,
            commands::search_items,
            // AI management
            commands::get_ai_status,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
            commands::activate_model,
            commands::retag_all,
            // Share intent
            commands::detect_feed,
            commands::get_pending_share,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// JNI function called from ShareBridge.kt when Android receives a share/view intent.
/// The symbol name encodes the Kotlin package+class: com.avinthakur080.pulse_rs.ShareBridge.onShareUrl
/// Note: underscores in package names are encoded as _1 in JNI.
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn Java_com_avinthakur080_pulse_1rs_ShareBridge_onShareUrl(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    url: jni::objects::JString,
) {
    let url: String = match env.get_string(&url) {
        Ok(s) => s.into(),
        Err(_) => return,
    };
    if url.is_empty() {
        return;
    }

    if let Some(handle) = APP_HANDLE.get() {
        let _ = handle.emit(
            "share://incoming-url",
            crate::models::IncomingShareEvent { url },
        );
    } else {
        // App not fully initialized yet (cold start) — store for later drain
        if let Some(pending) = PENDING_SHARE.get() {
            if let Ok(mut lock) = pending.lock() {
                *lock = Some(url);
            }
        }
    }
}
