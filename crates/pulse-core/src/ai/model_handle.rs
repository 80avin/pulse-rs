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
        let model = self.inner.read().ok()?.clone();
        if let Some(arc) = model {
            self.last_used.store(now_secs(), Ordering::Relaxed);
            return Some(arc);
        }
        if !self.pending_reload.swap(true, Ordering::AcqRel) {
            let handle = self.clone();
            tokio::task::spawn_blocking(move || {
                let loaded = (handle.loader)();
                if loaded.is_some() && handle.inner.read().is_ok_and(|x| x.is_some()) {
                    handle.pending_reload.store(false, Ordering::Release);
                    return;
                }
                if let Some(model) = loaded {
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

    pub fn store(&self, model: Arc<T>) {
        *self.inner.write().ok().expect("model_handle lock poisoned") = Some(model);
        self.last_used.store(now_secs(), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        *self.inner.write().ok().expect("model_handle lock poisoned") = None;
    }

    pub fn is_loaded(&self) -> bool {
        self.inner.read().ok().map(|x| x.is_some()).unwrap_or(false)
    }

    pub fn idle_drop(&self, threshold: Duration) -> bool {
        let last = self.last_used.load(Ordering::Relaxed);
        let now = now_secs();
        if now.saturating_sub(last) < threshold.as_secs() {
            return false;
        }
        let mut guard = self.inner.write().ok().expect("model_handle lock poisoned");
        if guard.is_none() {
            return false;
        }
        let last = self.last_used.load(Ordering::Relaxed);
        if now.saturating_sub(last) < threshold.as_secs() {
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
