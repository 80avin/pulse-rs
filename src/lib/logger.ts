const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

type Level = 'Error' | 'Warn' | 'Info' | 'Debug';

async function log(level: Level, message: string, context?: unknown): Promise<void> {
  const ctx = context !== undefined ? JSON.stringify(context) : undefined;
  if (IS_TAURI) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('log_from_frontend', { level, message, context: ctx });
    } catch {
      // Last-resort fallback: if the IPC bridge itself fails, write to console
      const fn_ = level === 'Error' ? console.error : level === 'Warn' ? console.warn : console.log;
      fn_(`[pulse/${level}]`, message, ctx ?? '');
    }
  } else {
    const fn_ = level === 'Error' ? console.error : level === 'Warn' ? console.warn : console.log;
    fn_(`[pulse/${level}]`, message, ctx ?? '');
  }
}

export const logger = {
  error: (message: string, context?: unknown) => log('Error', message, context),
  warn:  (message: string, context?: unknown) => log('Warn',  message, context),
  info:  (message: string, context?: unknown) => log('Info',  message, context),
  debug: (message: string, context?: unknown) => log('Debug', message, context),
};
