export type MarkReadMode = 'open' | 'never';
export type SyncInterval = 5 | 15 | 30 | 60;

const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

const KEY = 'pulse:settings';

function loadLocal(): Record<string, unknown> {
  if (typeof localStorage === 'undefined') return {};
  try { return JSON.parse(localStorage.getItem(KEY) ?? '{}'); }
  catch { return {}; }
}

const saved = loadLocal();

export const settings = $state({
  density:             (saved.density             ?? 'normal') as 'dense' | 'normal' | 'roomy',
  markReadOn:          (saved.markReadOn          ?? 'open')   as MarkReadMode,
  syncIntervalMin:     (saved.syncIntervalMin     ?? 15)       as SyncInterval,
  wifiOnly:            (saved.wifiOnly            ?? false)    as boolean,
  backgroundSync:      (saved.backgroundSync      ?? true)     as boolean,
  aiTagging:           (saved.aiTagging           ?? true)     as boolean,
  confidenceThreshold: (saved.confidenceThreshold ?? 0.5)     as number,
  notifyHighSignal:    (saved.notifyHighSignal     ?? false)   as boolean,
  notifySaved:         (saved.notifySaved          ?? false)   as boolean,
});

// Load from Tauri backend on startup (overrides localStorage if present)
if (IS_TAURI) {
  (async () => {
    try {
      const s = await tauriInvoke<typeof settings>('get_settings');
      settings.density             = s.density as typeof settings.density;
      settings.markReadOn          = s.markReadOn as MarkReadMode;
      settings.syncIntervalMin     = s.syncIntervalMin as SyncInterval;
      settings.wifiOnly            = s.wifiOnly;
      settings.backgroundSync      = s.backgroundSync;
      settings.aiTagging           = s.aiTagging;
      settings.confidenceThreshold = s.confidenceThreshold;
      settings.notifyHighSignal    = s.notifyHighSignal;
      settings.notifySaved         = s.notifySaved;
    } catch (e) {
      console.error('[pulse] failed to load settings from backend:', e);
    }
  })();
}

// Persist every change to localStorage + Tauri backend
$effect.root(() => {
  $effect(() => {
    const snap = {
      density:             settings.density,
      markReadOn:          settings.markReadOn,
      syncIntervalMin:     settings.syncIntervalMin,
      wifiOnly:            settings.wifiOnly,
      backgroundSync:      settings.backgroundSync,
      aiTagging:           settings.aiTagging,
      confidenceThreshold: settings.confidenceThreshold,
      notifyHighSignal:    settings.notifyHighSignal,
      notifySaved:         settings.notifySaved,
    };
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(KEY, JSON.stringify(snap));
    }
    if (IS_TAURI) {
      tauriInvoke('save_settings', { settings: snap }).catch(console.error);
    }
  });
});
