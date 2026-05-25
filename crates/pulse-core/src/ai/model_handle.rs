use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// A lazily-reloadable, idle-unloadable handle to an AI model.
///
/// Wraps `Arc<RwLock<Option<Arc<T>>>>` with access tracking and a loader closure.
/// The `snapshot()` method updates `last_used` and, when the model slot is empty,
/// spawns a background blocking thread to reload from disk — so the *next* tagging
/// request will find the model ready without stalling the current one.
///
/// `idle_drop(threshold)` is called periodically by a janitor task to evict models
/// that haven't been used recently, freeing memory on devices where that matters.
pub struct ModelHandle<T> {
    inner: Arc<RwLock<Option<Arc<T>>>>,
    last_used: Arc<AtomicU64>,
    loader: Arc<dyn Fn() -> Option<Arc<T>> + Send + Sync>,
    pending_reload: Arc<AtomicBool>,
}

impl<T> Clone for ModelHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            last_used: self.last_used.clone(),
            loader: self.loader.clone(),
            pending_reload: self.pending_reload.clone(),
        }
    }
}

impl<T: Send + Sync + 'static> ModelHandle<T> {
    /// Create a new handle. `loader` is called on a blocking thread whenever the
    /// model needs to be (re)loaded after an idle eviction.
    pub fn new(loader: Arc<dyn Fn() -> Option<Arc<T>> + Send + Sync>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(None)),
            last_used: Arc::new(AtomicU64::new(now_secs())),
            loader,
            pending_reload: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Read the current model and update the last-used timestamp.
    ///
    /// If the model slot is empty, triggers a background reload (once; no double-spawn)
    /// and returns `None` for this call. Subsequent calls will return the loaded model.
    pub fn snapshot(&self) -> Option<Arc<T>> {
        let model = self.inner.read().unwrap().clone();
        if let Some(arc) = model {
            self.last_used.store(now_secs(), Ordering::Relaxed);
            return Some(arc);
        }
        // Trigger a background reload if no reload is already in flight.
        if !self.pending_reload.swap(true, Ordering::AcqRel) {
            let handle = self.clone();
            tokio::task::spawn_blocking(move || {
                if let Some(model) = (handle.loader)() {
                    handle.store(model);
                    tracing::info!("Model reloaded on demand");
                } else {
                    tracing::debug!("No active model configured; skipping on-demand reload");
                }
                handle.pending_reload.store(false, Ordering::Release);
            });
        }
        None
    }

    /// Store a loaded model. Updates `last_used` to prevent immediate idle eviction.
    /// Called from init background threads and from hot-reload commands.
    pub fn store(&self, model: Arc<T>) {
        *self.inner.write().unwrap() = Some(model);
        self.last_used.store(now_secs(), Ordering::Relaxed);
    }

    /// Clear the model from memory without affecting the active-model pointer on disk.
    /// Call this from `remove_*_model` paths that also delete the files.
    pub fn clear(&self) {
        *self.inner.write().unwrap() = None;
    }

    /// Whether a model is currently in memory.
    pub fn is_loaded(&self) -> bool {
        self.inner.read().unwrap().is_some()
    }

    /// Evict the model if it has been idle for longer than `threshold`.
    ///
    /// Called by the janitor task; does nothing if the slot is already empty.
    /// Returns `true` if the model was evicted.
    pub fn idle_drop(&self, threshold: Duration) -> bool {
        let last = self.last_used.load(Ordering::Relaxed);
        let now = now_secs();
        if now.saturating_sub(last) < threshold.as_secs() {
            return false;
        }
        let mut guard = self.inner.write().unwrap();
        if guard.is_none() {
            return false;
        }
        *guard = None;
        tracing::info!(
            idle_secs = now.saturating_sub(last),
            "Model evicted after idle timeout"
        );
        true
    }
}
