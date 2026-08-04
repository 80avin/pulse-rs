import { IS_TAURI, tauriInvoke, items, sources, storeReady, reloadItems, reloadSources, reloadGroups, reloadDbStats } from './data.svelte';
import { reloadTagStats } from './ai.svelte';
import { logger } from '../logger';

// --- Mock sync pool (browser dev only) ---
const SYNC_POOL = [
  { id: 'syn01', src: 'hn-front', kind: 'link', domain: 'matklad.github.io', title: 'One Hundred Thousand Lines of Rust', author: 'matklad', age: '1m', score: 891, n: 143, tags: ['technical', 'deepdive'], read: false, saved: false, body: "" },
  { id: 'syn02', src: 'r-localllama', kind: 'text', title: 'Gemma-3n achieves 90.1 on MMLU-Pro at 4B params', author: 'u/mlbench', age: '3m', score: 1204, n: 87, tags: ['ai/ml', 'research'], read: false, saved: false, body: '' },
  { id: 'syn03', src: 'rss-julia', kind: 'link', domain: 'jvns.ca', title: 'Some fun things you can do with strace', author: 'b0rk', age: '5m', score: 0, n: 0, tags: ['tutorial', 'technical'], read: false, saved: false, body: '' },
] as const;

let syncPoolIdx = 0;
let idCounter = 0;

// --- Reactive state ---
export const syncState = $state({ lastSyncAt: 'never', lastNewCount: 0, syncing: false });

export async function doSync(): Promise<void> {
  if (syncState.syncing || storeReady.loading) return;
  syncState.syncing = true;
  try {
    if (IS_TAURI) {
      const result = await tauriInvoke<{ newCount: number; error: string | null }>('sync_all');
      await Promise.all([reloadItems(), reloadSources(), reloadGroups(), reloadDbStats(), reloadTagStats()]);
      const t = new Date();
      syncState.lastSyncAt = `${String(t.getHours()).padStart(2, '0')}:${String(t.getMinutes()).padStart(2, '0')}`;
      syncState.lastNewCount = result.newCount;
    } else {
      await new Promise(r => setTimeout(r, 1200));
      for (const s of sources) { if (s.status !== 'error') s.lastSync = 'just now'; }
      const batch = Math.min(3, SYNC_POOL.length);
      for (let i = 0; i < batch; i++) {
        const tmpl = SYNC_POOL[syncPoolIdx % SYNC_POOL.length];
        syncPoolIdx++;
        items.unshift({ ...tmpl, id: `sync-${++idCounter}-${tmpl.id}`, age: 'just now' } as any);
      }
      const t = new Date();
      syncState.lastSyncAt = `${String(t.getHours()).padStart(2, '0')}:${String(t.getMinutes()).padStart(2, '0')}`;
      syncState.lastNewCount = batch;
    }
  } catch (e) {
    logger.error('sync_all failed', e);
  } finally {
    syncState.syncing = false;
  }
}
