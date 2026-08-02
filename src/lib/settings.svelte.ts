import { logger } from './logger';

export type MarkReadMode = 'open' | 'never';
export type SyncInterval = 5 | 15 | 30 | 60;

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
  theme:              (saved.theme              ?? 'system')  as 'system' | 'dark' | 'light',
  accentColor:        (saved.accentColor        ?? 'cyan')  as string,
  density:             (saved.density             ?? 'normal') as 'dense' | 'normal' | 'roomy',
  markReadOn:          (saved.markReadOn          ?? 'open')   as MarkReadMode,
  syncIntervalMin:     (saved.syncIntervalMin     ?? 15)       as SyncInterval,
  wifiOnly:            (saved.wifiOnly            ?? false)    as boolean,
  backgroundSync:      (saved.backgroundSync      ?? true)     as boolean,
  notifyHighSignal:    (saved.notifyHighSignal     ?? false)   as boolean,
  notifySaved:         (saved.notifySaved          ?? false)   as boolean,
  verboseLogging:      (saved.verboseLogging       ?? false)   as boolean,
});

// Called from +layout.svelte onMount — same timing guarantee as initStore().
// Settings have a localStorage fallback, so failure here is non-fatal.
export async function initSettings(): Promise<void> {
  if (!IS_TAURI) return;
  const MAX_ATTEMPTS = 5;
  const ATTEMPT_TIMEOUT_MS = 4000;
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    if (attempt > 0) {
      const delay = attempt < 3 ? 300 * attempt : 1000 * (attempt - 1);
      await new Promise(r => setTimeout(r, delay));
    }
    try {
      const s = await Promise.race([
        tauriInvoke<typeof settings>('get_settings'),
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error('settings timeout')), ATTEMPT_TIMEOUT_MS)
        ),
      ]);
      // Only apply backend values that are actually present — never overwrite
      // localStorage defaults with undefined. This is essential for new fields
      // (theme, accentColor) that the Rust backend may not return yet.
      if (s.theme !== undefined && s.theme !== null)              settings.theme              = s.theme as 'system' | 'dark' | 'light';
      if (s.accentColor !== undefined && s.accentColor !== null)  settings.accentColor        = s.accentColor as string;
      if (s.density !== undefined && s.density !== null)          settings.density            = s.density as typeof settings.density;
      if (s.markReadOn !== undefined && s.markReadOn !== null)    settings.markReadOn          = s.markReadOn as MarkReadMode;
      if (s.syncIntervalMin !== undefined && s.syncIntervalMin !== null) settings.syncIntervalMin = s.syncIntervalMin as SyncInterval;
      if (s.wifiOnly !== undefined && s.wifiOnly !== null)        settings.wifiOnly            = s.wifiOnly;
      if (s.backgroundSync !== undefined && s.backgroundSync !== null) settings.backgroundSync = s.backgroundSync;
      if (s.notifyHighSignal !== undefined && s.notifyHighSignal !== null) settings.notifyHighSignal = s.notifyHighSignal;
      if (s.notifySaved !== undefined && s.notifySaved !== null)  settings.notifySaved         = s.notifySaved;
      if (s.verboseLogging !== undefined && s.verboseLogging !== null) settings.verboseLogging = s.verboseLogging;
      return;
    } catch (e) {
      logger.warn(`settings init attempt ${attempt + 1} failed`, e);
    }
  }
}

// Persist every change to localStorage + Tauri backend
$effect.root(() => {
  $effect(() => {
    const snap = {
      theme:              settings.theme,
      accentColor:        settings.accentColor,
      density:             settings.density,
      markReadOn:          settings.markReadOn,
      syncIntervalMin:     settings.syncIntervalMin,
      wifiOnly:            settings.wifiOnly,
      backgroundSync:      settings.backgroundSync,
      notifyHighSignal:    settings.notifyHighSignal,
      notifySaved:         settings.notifySaved,
      verboseLogging:      settings.verboseLogging,
    };
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(KEY, JSON.stringify(snap));
    }
    if (IS_TAURI) {
      tauriInvoke('save_settings', { settings: snap }).catch(e => logger.warn('save_settings failed', e));
    }
  });

  // Apply the log filter live whenever verboseLogging changes — separate effect
  // so the Rust filter update fires immediately and independently of save_settings.
  $effect(() => {
    const verbose = settings.verboseLogging;
    if (IS_TAURI) {
      tauriInvoke('set_log_level', { verbose }).catch(() => {});
    }
  });
});
