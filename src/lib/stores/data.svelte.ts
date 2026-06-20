import { ITEMS as MOCK_ITEMS, SOURCES as MOCK_SOURCES, GROUPS as MOCK_GROUPS } from '../mock-data';
import type { FeedItem, Source, Group } from '../types';
import { logger } from '../logger';
import { settings } from '../settings.svelte';

// ── ⚠️ MUTATIONS ONLY THROUGH FUNCTIONS BELOW ─────────────────────────────
// Never splice/push/reassign arrays directly from components.
// Use the exported mutation functions (markRead, toggleSaved, etc.).

export const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// --- Backend types (camelCase from Rust serde) ---
interface BackendItem {
  id: string; sourceId: string; sourceName: string; title: string; url: string;
  body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null;
  publishedAt: string; read: boolean; saved: boolean; hidden: boolean;
  score: number | null; n: number; tags: string[]; signal: number;
  ogImage: string | null; note: string | null;
}

interface BackendSource {
  id: string; name: string; url: string; kind: 'hn' | 'reddit' | 'rss';
  group: string; unread: number; itemCount: number; avgLatencyMs: number | null;
  lastSync: string | null; enabled: boolean; failureStreak: number; hue: number | null;
}

export interface BackendGroup { id: string; name: string; n: number; }

// --- Adapters ---
export function ageLabel(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const m = Math.floor(diff / 60000);
  if (m < 1) return 'now';
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

export function domainOf(url: string): string {
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return ''; }
}

export function adaptItem(b: BackendItem): FeedItem {
  const isHnSelf = b.url.includes('news.ycombinator.com/item');
  return {
    id: b.id, src: b.sourceId,
    kind: (b.url && !isHnSelf) ? 'link' : 'text',
    title: b.title, url: b.url, body: b.body,
    bodyHtml: b.bodyHtml ?? undefined,
    externalUrl: b.externalUrl ?? undefined,
    author: b.author ?? '', age: ageLabel(b.publishedAt),
    score: b.score ?? 0, n: b.n, tags: b.tags, aiScore: b.signal,
    read: b.read, saved: b.saved,
    domain: domainOf(b.url),
    ogImage: b.ogImage ?? null, note: b.note ?? undefined,
  };
}

function adaptSource(b: BackendSource): Source {
  const secsAgo = b.lastSync ? (Date.now() - new Date(b.lastSync).getTime()) / 1000 : Infinity;
  const status: Source['status'] =
    b.failureStreak >= 3 ? 'error' : secsAgo < 3600 ? 'ok' : 'stale';
  return {
    id: b.id, kind: b.kind, name: b.name, url: b.url, host: domainOf(b.url),
    items: b.itemCount, unread: b.unread,
    lastSync: b.lastSync ? `${ageLabel(b.lastSync)} ago` : 'never',
    status, latencyMs: Math.round(b.avgLatencyMs ?? 0),
    group: b.group, failureStreak: b.failureStreak, hue: b.hue ?? undefined,
  };
}

// --- Reactive state ---
export const items   = $state<FeedItem[]>(IS_TAURI ? [] : MOCK_ITEMS.map(i => ({ ...i })));
export const sources = $state<Source[]>(IS_TAURI ? [] : MOCK_SOURCES.map(s => ({ ...s })));
export const groups  = $state<Group[]>(IS_TAURI ? [] : MOCK_GROUPS.map(g => ({ ...g })));

export const storeReady = $state({ loading: true, error: false });

export interface ColdstartTiming {
  attempt: number; totalMs: number; waitMs: number; ipcMs: number; adaptMs: number;
  itemCount: number; sourceCount: number; groupCount: number;
}

export const coldstartTiming = $state<{ data: ColdstartTiming | null }>({ data: null });

export const dbStats = $state({
  totalItems: 0, unreadItems: 0, savedItems: 0, totalSources: 0, dbSizeKb: 0, tagCount: 0,
});

// --- Reload helpers (used by data mutations and sync module) ---
export async function reloadItems(): Promise<void> {
  const { timelineFilter, pageCounts } = await import('./timeline.svelte');
  const page = await tauriInvoke<{ items: BackendItem[]; nextCursor: { publishedAt: number; itemId: string } | null; counts: { total: number; unread: number; saved: number; signal: number } }>('get_items_page', {
    groupId: timelineFilter.groupId ?? null,
    feedId: timelineFilter.feedId ?? null,
    tag: timelineFilter.tag ?? null,
    isRead: timelineFilter.isRead,
    isSaved: timelineFilter.isSaved,
    signalThreshold: settings.confidenceThreshold,
    limit: 100,
    cursor: null,
  });
  items.splice(0, items.length, ...page.items.map(adaptItem));
  const { loadingMore, hasPrecedingItems } = await import('./timeline.svelte');
  loadingMore.cursor = page.nextCursor ?? null;
  hasPrecedingItems.value = false;
  if (page.counts) {
    pageCounts.total = page.counts.total;
    pageCounts.unread = page.counts.unread;
    pageCounts.saved = page.counts.saved;
    pageCounts.signal = page.counts.signal;
  }
}

export async function reloadSources(): Promise<void> {
  const bs = await tauriInvoke<BackendSource[]>('get_sources');
  sources.splice(0, sources.length, ...bs.map(adaptSource));
}

export async function reloadGroups(): Promise<void> {
  const bg = await tauriInvoke<BackendGroup[]>('get_groups');
  groups.splice(0, groups.length, ...bg);
}

export async function reloadDbStats(): Promise<void> {
  if (!IS_TAURI) return;
  const s = await tauriInvoke<{ totalItems: number; unreadItems: number; savedItems: number; totalSources: number; dbSizeKb: number; tagCount: number }>('get_db_stats');
  dbStats.totalItems = s.totalItems;
  dbStats.unreadItems = s.unreadItems;
  dbStats.savedItems = s.savedItems;
  dbStats.totalSources = s.totalSources;
  dbStats.dbSizeKb = s.dbSizeKb;
  dbStats.tagCount = s.tagCount;
}

// --- initStore (cold-start retry loop) ---
let _initStarted = false;

export async function initStore(): Promise<void> {
  if (_initStarted) return;
  _initStarted = true;
  if (!IS_TAURI) { storeReady.loading = false; return; }

  const t0 = performance.now();
  const TIMEOUTS = [2000, 2000, 3000, 4000, 5000];
  const DELAYS   = [0, 200, 500, 1000, 2000];

  for (let attempt = 0; attempt < TIMEOUTS.length; attempt++) {
    if (attempt > 0) await new Promise(r => setTimeout(r, DELAYS[attempt]));
    const tAttempt = performance.now();
    try {
      const tInvoke = performance.now();
      const [page, bs, bg] = await Promise.race([
        Promise.all([
          tauriInvoke<{ items: BackendItem[]; nextCursor: { publishedAt: number; itemId: string } | null; counts: { total: number; unread: number; saved: number; signal: number } }>('get_items_page', { limit: 100 }),
          tauriInvoke<BackendSource[]>('get_sources'),
          tauriInvoke<BackendGroup[]>('get_groups'),
        ]),
        new Promise<never>((_, reject) => setTimeout(() => reject(new Error('init timeout')), TIMEOUTS[attempt])),
      ]);
      const tInvokeDone = performance.now();
      items.splice(0, items.length, ...page.items.map(adaptItem));
      sources.splice(0, sources.length, ...bs.map(adaptSource));
      groups.splice(0, groups.length, ...bg);
      const { loadingMore, pageCounts } = await import('./timeline.svelte');
      loadingMore.cursor = page.nextCursor ?? null;
      if (page.counts) {
        pageCounts.total = page.counts.total;
        pageCounts.unread = page.counts.unread;
        pageCounts.saved = page.counts.saved;
        pageCounts.signal = page.counts.signal;
      }
      const tAdaptDone = performance.now();
      storeReady.loading = false;
      storeReady.error = false;
      coldstartTiming.data = {
        attempt, totalMs: Math.round(tAdaptDone - t0),
        waitMs: Math.round(tAttempt - t0), ipcMs: Math.round(tInvokeDone - tInvoke),
        adaptMs: Math.round(tAdaptDone - tInvokeDone),
        itemCount: page.items.length, sourceCount: bs.length, groupCount: bg.length,
      };
      logger.info('coldstart: initStore complete', coldstartTiming.data);
      // Fire-and-forget AI + stats init (no circular import: dynamic)
      import('./ai.svelte').then(m => {
        m.reloadAiInfo().catch(e => logger.warn('ai info failed', e));
        m.reloadAiStats().catch(e => logger.warn('ai stats failed', e));
      });
      reloadDbStats().catch(e => logger.warn('db stats failed', e));
      return;
    } catch (e) {
      logger.warn(`coldstart: initStore attempt ${attempt + 1} failed`, e);
    }
  }
  storeReady.loading = false;
  storeReady.error = true;
}

if (IS_TAURI) initStore();

export function loadMockData() {
  items.splice(0, items.length, ...MOCK_ITEMS.map(i => ({ ...i })));
  sources.splice(0, sources.length, ...MOCK_SOURCES.map(s => ({ ...s })));
  groups.splice(0, groups.length, ...MOCK_GROUPS.map(g => ({ ...g })));
}

// --- Public mutations ---

export async function markRead(id: string, read = true) {
  const { timelineFilter } = await import('./timeline.svelte');
  const item = items.find(i => i.id === id);
  const wasRead = item?.read;
  if (item) {
    item.read = read;
    const src = sources.find(s => s.id === item.src);
    if (src) {
      const delta = !wasRead && read ? -1 : wasRead && !read ? 1 : 0;
      if (delta !== 0) {
        src.unread = Math.max(0, src.unread + delta);
        for (const g of groups) {
          if (g.id === 'all' || g.id === src.group) {
            g.n = Math.max(0, g.n + delta);
          }
        }
      }
    }
  }
  if (IS_TAURI) {
    try {
      await tauriInvoke('mark_items_read', { ids: [id], read });
    } catch {
      if (item && wasRead !== undefined) {
        item.read = wasRead;
        const src = sources.find(s => s.id === item.src);
        if (src) {
          const delta = !wasRead && read ? -1 : wasRead && !read ? 1 : 0;
          if (delta !== 0) {
            src.unread = Math.max(0, src.unread - delta);
            for (const g of groups) {
              if (g.id === 'all' || g.id === src.group) {
                g.n = Math.max(0, g.n - delta);
              }
            }
          }
        }
      }
      return;
    }
    if (tlEvict(id, timelineFilter)) items.splice(items.findIndex(i => i.id === id), 1);
    await Promise.all([reloadDbStats(), reloadSources(), reloadGroups()]);
  }
}

export async function toggleSaved(id: string, note?: string) {
  const { timelineFilter } = await import('./timeline.svelte');
  const item = items.find(i => i.id === id);
  if (!item) {
    if (IS_TAURI) {
      await tauriInvoke('toggle_saved', { id, saved: true, note: note ?? undefined });
      await reloadDbStats();
    }
    return;
  }
  const wasSaved = item.saved;
  const wasNote = item.note;
  item.saved = !item.saved;
  if (note !== undefined) item.note = note;
  if (IS_TAURI) {
    try {
      await tauriInvoke('toggle_saved', { id, saved: item.saved, note: note ?? item.note });
    } catch {
      item.saved = wasSaved;
      item.note = wasNote;
      return;
    }
    if (tlEvict(id, timelineFilter)) items.splice(items.findIndex(i => i.id === id), 1);
    await Promise.all([reloadDbStats(), reloadSources()]);
  }
}

export async function markAllRead(ids: string[]) {
  const { timelineFilter } = await import('./timeline.svelte');
  for (const id of ids) {
    const item = items.find(i => i.id === id);
    if (item && !item.read) {
      item.read = true;
      const src = sources.find(s => s.id === item.src);
      if (src) {
        src.unread = Math.max(0, src.unread - 1);
        for (const g of groups) {
          if (g.id === 'all' || g.id === src.group) {
            g.n = Math.max(0, g.n - 1);
          }
        }
      }
    }
  }
  if (IS_TAURI) {
    try {
      await tauriInvoke('mark_items_read', { ids, read: true });
    } catch {
      await Promise.all([reloadItems(), reloadSources(), reloadGroups()]);
      return;
    }
    if (timelineFilter.isRead !== null) {
      const idSet = new Set(ids);
      for (let i = items.length - 1; i >= 0; i--) {
        if (idSet.has(items[i].id) && items[i].read !== timelineFilter.isRead) {
          items.splice(i, 1);
        }
      }
    }
    await Promise.all([reloadDbStats(), reloadSources(), reloadGroups()]);
  }
}

export async function markSourceRead(sourceId: string) {
  const { timelineFilter } = await import('./timeline.svelte');
  const markedIds: string[] = [];
  for (const item of items) {
    if (item.src === sourceId && !item.read) {
      item.read = true;
      markedIds.push(item.id);
    }
  }
  const src = sources.find(s => s.id === sourceId);
  const prevUnread = src?.unread ?? 0;
  if (src) {
    src.unread = 0;
    if (prevUnread > 0) {
      for (const g of groups) {
        if (g.id === 'all' || g.id === src.group) {
          g.n = Math.max(0, g.n - prevUnread);
        }
      }
    }
  }
  if (IS_TAURI) {
    try {
      await tauriInvoke('mark_source_read', { sourceId });
    } catch {
      const idSet = new Set(markedIds);
      for (const item of items) { if (idSet.has(item.id)) item.read = false; }
      if (src) {
        src.unread = prevUnread;
        for (const g of groups) {
          if (g.id === 'all' || g.id === src.group) {
            g.n = Math.max(0, g.n + prevUnread);
          }
        }
      }
      return;
    }
    if (timelineFilter.isRead !== null) {
      for (let i = items.length - 1; i >= 0; i--) {
        if (items[i].src === sourceId && items[i].read !== timelineFilter.isRead) {
          items.splice(i, 1);
        }
      }
    }
    await Promise.all([reloadDbStats(), reloadSources(), reloadGroups()]);
  }
}

export async function hideItem(id: string) {
  const idx = items.findIndex(i => i.id === id);
  const removed = idx !== -1 ? items[idx] : null;
  if (idx !== -1) items.splice(idx, 1);
  if (IS_TAURI) {
    try {
      await tauriInvoke('hide_item', { id });
      await Promise.all([reloadDbStats(), reloadSources(), reloadGroups()]);
    } catch {
      if (removed) items.splice(idx, 0, removed);
    }
  }
}

export async function addSource(name: string, url: string, kind: 'hn' | 'reddit' | 'rss', group: string, hue?: number): Promise<string> {
  const id = crypto.randomUUID();
  if (IS_TAURI) {
    await tauriInvoke('add_source', {
      source: { id, name, url, kind, group, unread: 0, lastSync: null, enabled: true, itemCount: 0, failureStreak: 0, hue: hue ?? null },
    });
    await Promise.all([reloadSources(), reloadGroups(), reloadDbStats()]);
  } else {
    sources.push({ id, kind, name, url, host: domainOf(url), items: 0, unread: 0, lastSync: 'never', status: 'stale', latencyMs: 0, group, failureStreak: 0, hue });
  }
  return id;
}

export async function updateSource(id: string, name: string, url: string, kind: 'hn' | 'reddit' | 'rss', group: string, hue?: number): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('update_source', { id, name, url, kind, group, hue: hue ?? null });
    await Promise.all([reloadSources(), reloadGroups()]);
  } else {
    const s = sources.find(s => s.id === id);
    if (s) { s.name = name; s.url = url; s.kind = kind; s.host = domainOf(url); s.group = group; if (hue !== undefined) s.hue = hue; }
  }
}

export interface FeedPreview { name: string; kind: string; }

export async function detectFeed(url: string): Promise<FeedPreview | null> {
  if (!IS_TAURI) return null;
  try { return await tauriInvoke<FeedPreview>('detect_feed', { url }); } catch { return null; }
}

export async function removeSource(id: string): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('delete_source', { id });
    for (let i = items.length - 1; i >= 0; i--) { if (items[i].src === id) items.splice(i, 1); }
    await Promise.all([reloadSources(), reloadGroups(), reloadDbStats()]);
  } else {
    const idx = sources.findIndex(s => s.id === id);
    if (idx !== -1) {
      for (let i = items.length - 1; i >= 0; i--) { if (items[i].src === id) items.splice(i, 1); }
      sources.splice(idx, 1);
    }
  }
}

export async function syncSource(sourceId: string): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('sync_source', { sourceId });
    await Promise.all([reloadItems(), reloadSources(), reloadGroups(), reloadDbStats()]);
  } else {
    const { doSync } = await import('./sync.svelte');
    await doSync();
  }
}

export async function clearItems(): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('clear_items');
    items.splice(0, items.length);
    await Promise.all([reloadSources(), reloadGroups(), reloadDbStats()]);
  } else {
    items.splice(0, items.length);
  }
}

export async function createGroup(name: string): Promise<void> {
  const id = name.toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
  if (!id) return;
  if (IS_TAURI) {
    await tauriInvoke('add_group', { id, name });
    await reloadGroups();
  } else {
    if (!groups.find(g => g.id === id)) groups.push({ id, name, n: 0 });
  }
}

export async function renameGroup(id: string, name: string): Promise<void> {
  const g = groups.find(g => g.id === id);
  const oldName = g?.name;
  if (g) g.name = name;
  if (IS_TAURI) {
    try { await tauriInvoke('rename_group', { id, name }); }
    catch { if (g) g.name = oldName ?? name; }
  }
}

export async function deleteGroup(id: string): Promise<void> {
  if (id === 'all') return;
  const oldGroups = new Map(sources.filter(s => s.group === id).map(s => [s.id, s.group]));
  for (const s of sources) { if (s.group === id) s.group = 'all'; }
  if (IS_TAURI) {
    try {
      await tauriInvoke('delete_group', { id });
      await Promise.all([reloadSources(), reloadGroups()]);
    } catch {
      for (const s of sources) { if (oldGroups.has(s.id)) s.group = oldGroups.get(s.id)!; }
    }
  } else {
    const idx = groups.findIndex(g => g.id === id);
    if (idx !== -1) groups.splice(idx, 1);
  }
}

// --- Internal helpers ---
function tlEvict(id: string, timelineFilter: { isRead: boolean | null; isSaved: boolean | null }): boolean {
  const item = items.find(i => i.id === id);
  if (!item) return false;
  const { isRead, isSaved } = timelineFilter;
  return (isRead !== null && item.read !== isRead) || (isSaved !== null && item.saved !== isSaved);
}
