import { ITEMS as MOCK_ITEMS, SOURCES as MOCK_SOURCES, GROUPS as MOCK_GROUPS } from './mock-data';
import type { FeedItem, Source, Group, AiStatus, ModelInfo } from './types';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

// --- Backend types (camelCase from Rust serde) ---
interface BackendItem {
  id: string;
  sourceId: string;
  sourceName: string;
  title: string;
  url: string;
  body: string;
  bodyHtml: string | null;
  externalUrl: string | null;
  author: string | null;
  publishedAt: string;
  read: boolean;
  saved: boolean;
  hidden: boolean;
  score: number | null;
  n: number;
  tags: string[];
  signal: number;
  ogImage: string | null;
}

interface BackendSource {
  id: string;
  name: string;
  url: string;
  kind: 'hn' | 'reddit' | 'rss';
  group: string;
  unread: number;
  itemCount: number;
  avgLatencyMs: number | null;
  lastSync: string | null;
  enabled: boolean;
  failureStreak: number;
}

interface BackendGroup { id: string; name: string; n: number; }

interface BackendAiStatus {
  modelLoaded: boolean;
  visionLoaded: boolean;
  fasttextLoaded: boolean;
  minimlLoaded: boolean;
  modelName: string | null;
  visionModelName: string | null;
  fasttextModelName: string | null;
  minimlModelName: string | null;
  taggingMode: string;
}

interface BackendModelInfo {
  id: string;
  name: string;
  description: string;
  sizeMb: number;
  downloaded: boolean;
  active: boolean;
  kind: string;
}

// --- Adapters ---
function ageLabel(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const m = Math.floor(diff / 60000);
  if (m < 1) return 'now';
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

function domainOf(url: string): string {
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return ''; }
}

function adaptItem(b: BackendItem): FeedItem {
  const isHnSelf = b.url.includes('news.ycombinator.com/item');
  return {
    id: b.id,
    src: b.sourceId,
    kind: (b.url && !isHnSelf) ? 'link' : 'text',
    title: b.title,
    url: b.url,
    body: b.body,
    bodyHtml: b.bodyHtml ?? undefined,
    externalUrl: b.externalUrl ?? undefined,
    author: b.author ?? '',
    age: ageLabel(b.publishedAt),
    score: b.score ?? 0,
    n: b.n,
    tags: b.tags,
    aiScore: b.signal,
    read: b.read,
    saved: b.saved,
    domain: domainOf(b.url),
    ogImage: b.ogImage ?? null,
  };
}

function adaptSource(b: BackendSource): Source {
  const secsAgo = b.lastSync
    ? (Date.now() - new Date(b.lastSync).getTime()) / 1000
    : Infinity;
  const status: Source['status'] =
    b.failureStreak >= 3 ? 'error' :
    secsAgo < 3600       ? 'ok'    : 'stale';
  return {
    id: b.id,
    kind: b.kind,
    name: b.name,
    url: b.url,
    host: domainOf(b.url),
    items: b.itemCount,
    unread: b.unread,
    lastSync: b.lastSync ? `${ageLabel(b.lastSync)} ago` : 'never',
    status,
    latencyMs: Math.round(b.avgLatencyMs ?? 0),
    group: b.group,
    failureStreak: b.failureStreak,
  };
}

// --- Reactive state ---
export const items   = $state<FeedItem[]>(IS_TAURI ? [] : MOCK_ITEMS.map(i => ({ ...i })));
export const sources = $state<Source[]>(IS_TAURI ? [] : MOCK_SOURCES.map(s => ({ ...s })));
export const groups  = $state<Group[]>(IS_TAURI ? [] : MOCK_GROUPS.map(g => ({ ...g })));
export const syncState = $state({ lastSyncAt: 'never', lastNewCount: 0, syncing: false });
export const aiStatus = $state<AiStatus>({
  modelLoaded: false, visionLoaded: false, fasttextLoaded: false, minimlLoaded: false,
  modelName: null, visionModelName: null, fasttextModelName: null, minimlModelName: null,
  taggingMode: 'loading',
});
export const models = $state<ModelInfo[]>([]);

export const taggingProgress = $state({ active: false, tagged: 0, total: 0 });

/** Pagination state: cursor from last `get_items_page` call and in-flight flag. */
export const loadingMore = $state({
  active: false,
  cursor: null as { publishedAt: number; itemId: string } | null,
});

/** Call once from layout to wire the global tagging-progress event listener. */
export async function setupTaggingListener(): Promise<() => void> {
  if (!IS_TAURI) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<{ tagged: number; total: number; done: boolean }>(
    'ai://tagging-progress',
    (ev) => {
      const { tagged, total, done } = ev.payload;
      if (done) {
        taggingProgress.active = false;
        taggingProgress.tagged = total;
        taggingProgress.total  = total;
      } else {
        taggingProgress.active = true;
        taggingProgress.tagged = tagged;
        taggingProgress.total  = total;
      }
    }
  );
}

// --- Backend page response type ---
interface BackendPage {
  items: BackendItem[];
  nextCursor: { publishedAt: number; itemId: string } | null;
}

// --- Internal reload helpers ---
async function reloadItems(): Promise<void> {
  const page = await tauriInvoke<BackendPage>('get_items_page', { limit: 100 });
  items.splice(0, items.length, ...page.items.map(adaptItem));
  loadingMore.cursor = page.nextCursor ?? null;
}

async function reloadSources(): Promise<void> {
  const bs = await tauriInvoke<BackendSource[]>('get_sources');
  sources.splice(0, sources.length, ...bs.map(adaptSource));
}

async function reloadGroups(): Promise<void> {
  const bg = await tauriInvoke<BackendGroup[]>('get_groups');
  groups.splice(0, groups.length, ...bg);
}

async function reloadAiStatus(): Promise<void> {
  const s = await tauriInvoke<BackendAiStatus>('get_ai_status');
  aiStatus.modelLoaded      = s.modelLoaded;
  aiStatus.visionLoaded     = s.visionLoaded;
  aiStatus.fasttextLoaded   = s.fasttextLoaded;
  aiStatus.minimlLoaded     = s.minimlLoaded;
  aiStatus.modelName        = s.modelName;
  aiStatus.visionModelName  = s.visionModelName;
  aiStatus.fasttextModelName = s.fasttextModelName;
  aiStatus.minimlModelName  = s.minimlModelName;
  aiStatus.taggingMode      = s.taggingMode;
}

async function reloadModelsInternal(): Promise<void> {
  const ms = await tauriInvoke<BackendModelInfo[]>('list_models');
  models.splice(0, models.length, ...ms.map(m => ({ ...m, kind: m.kind as 'nli' | 'vision' | 'fasttext' | 'miniml' })));
}

// --- Init from DB on startup ---
if (IS_TAURI) {
  (async () => {
    try {
      const [page, bs, bg] = await Promise.all([
        tauriInvoke<BackendPage>('get_items_page', { limit: 100 }),
        tauriInvoke<BackendSource[]>('get_sources'),
        tauriInvoke<BackendGroup[]>('get_groups'),
      ]);
      items.splice(0, items.length, ...page.items.map(adaptItem));
      loadingMore.cursor = page.nextCursor ?? null;
      sources.splice(0, sources.length, ...bs.map(adaptSource));
      groups.splice(0, groups.length, ...bg);
    } catch (e) {
      console.error('[pulse] init failed:', e);
    }
    // Load AI status independently so a model init failure doesn't block the feed
    reloadAiStatus().catch(e => console.error('[pulse] ai status failed:', e));
    reloadModelsInternal().catch(e => console.error('[pulse] list_models failed:', e));
  })();
}

export function loadMockData() {
  items.splice(0, items.length, ...MOCK_ITEMS.map(i => ({ ...i })));
  sources.splice(0, sources.length, ...MOCK_SOURCES.map(s => ({ ...s })));
  groups.splice(0, groups.length, ...MOCK_GROUPS.map(g => ({ ...g })));
}

// --- Mock sync pool (browser dev only) ---
const SYNC_POOL: FeedItem[] = [
  { id: 'syn01', src: 'hn-front',      kind: 'link', domain: 'matklad.github.io',   title: 'One Hundred Thousand Lines of Rust',                                                   author: 'matklad',   age: '1m',  score: 891,  n: 143, tags: ['technical', 'deepdive'],  aiScore: 0.96, read: false, saved: false, body: "A retrospective on writing a large Rust codebase -- what worked, what didn't, and what I'd do differently." },
  { id: 'syn02', src: 'r-localllama',  kind: 'text',                                title: 'Gemma-3n achieves 90.1 on MMLU-Pro at 4B params -- reproducible eval harness inside', author: 'u/mlbench', age: '3m',  score: 1204, n: 87,  tags: ['ai/ml', 'research'],     aiScore: 0.91, read: false, saved: false, body: 'Full eval harness, methodology, and reproduction instructions attached.' },
  { id: 'syn03', src: 'rss-julia',     kind: 'link', domain: 'jvns.ca',             title: 'Some fun things you can do with strace',                                               author: 'b0rk',      age: '5m',  score: 0,    n: 0,   tags: ['tutorial', 'technical'], aiScore: 0.89, read: false, saved: false, body: "strace is surprisingly good for debugging things that aren't C programs." },
  { id: 'syn04', src: 'hn-newest',     kind: 'link', domain: 'rachelbythebay.com',  title: 'The perils of floating point: NaN silently corrupts database aggregates',              author: 'rachel',    age: '7m',  score: 642,  n: 201, tags: ['technical', 'research'], aiScore: 0.94, read: false, saved: false, body: "NaN propagation through SQL aggregate functions is an under-documented footgun." },
  { id: 'syn05', src: 'r-rust',        kind: 'text',                                title: 'Improving compile times: incremental vs. sccache vs. nothing',                        author: 'u/rustfan', age: '9m',  score: 278,  n: 44,  tags: ['technical'],             aiScore: 0.78, read: false, saved: false, body: 'I benchmarked three approaches on a 70k-line codebase. Results are surprising.' },
];
let syncPoolIdx = 0;
let idCounter   = 0;

// --- Public mutations ---

export async function doSync(): Promise<void> {
  if (syncState.syncing) return;
  syncState.syncing = true;
  try {
    if (IS_TAURI) {
      const result = await tauriInvoke<{ newCount: number; error: string | null }>('sync_all');
      await Promise.all([reloadItems(), reloadSources(), reloadGroups()]);
      const t = new Date(); syncState.lastSyncAt = `${String(t.getHours()).padStart(2,'0')}:${String(t.getMinutes()).padStart(2,'0')}`;
      syncState.lastNewCount = result.newCount;
    } else {
      await new Promise(r => setTimeout(r, 1200));
      for (const s of sources) { if (s.status !== 'error') s.lastSync = 'just now'; }
      const batch = Math.min(3, SYNC_POOL.length);
      for (let i = 0; i < batch; i++) {
        const tmpl = SYNC_POOL[syncPoolIdx % SYNC_POOL.length];
        syncPoolIdx++;
        items.unshift({ ...tmpl, id: `sync-${++idCounter}-${tmpl.id}`, age: 'just now' });
      }
      const t = new Date(); syncState.lastSyncAt = `${String(t.getHours()).padStart(2,'0')}:${String(t.getMinutes()).padStart(2,'0')}`;
      syncState.lastNewCount = batch;
    }
  } catch (e) {
    console.error('[pulse] sync failed:', e);
  } finally {
    syncState.syncing = false;
  }
}

/** Fetch the next page of items and append them to the `items` array. */
export async function loadMoreItems(groupId?: string): Promise<void> {
  if (!IS_TAURI || !loadingMore.cursor || loadingMore.active) return;
  loadingMore.active = true;
  try {
    const page = await tauriInvoke<BackendPage>('get_items_page', {
      groupId: groupId ?? null,
      limit: 100,
      cursor: loadingMore.cursor
        ? { publishedAt: loadingMore.cursor.publishedAt, itemId: loadingMore.cursor.itemId }
        : null,
    });
    items.push(...page.items.map(adaptItem));
    loadingMore.cursor = page.nextCursor ?? null;
  } catch (e) {
    console.error('[pulse] loadMoreItems failed:', e);
  } finally {
    loadingMore.active = false;
  }
}

export function markRead(id: string, read = true) {
  const item = items.find(i => i.id === id);
  if (!item) return;
  const wasRead = item.read;
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
  if (IS_TAURI) {
    tauriInvoke('mark_items_read', { ids: [id], read }).catch(console.error);
  }
}

export function toggleSaved(id: string) {
  const item = items.find(i => i.id === id);
  if (!item) return;
  item.saved = !item.saved;
  if (IS_TAURI) {
    tauriInvoke('toggle_saved', { id, saved: item.saved }).catch(console.error);
  }
}

export function markAllRead(ids: string[]) {
  for (const id of ids) markRead(id, true);
}

export function markSourceRead(sourceId: string) {
  let markedCount = 0;
  for (const item of items) {
    if (item.src === sourceId && !item.read) {
      item.read = true;
      markedCount++;
    }
  }
  const src = sources.find(s => s.id === sourceId);
  if (src) {
    const prevUnread = src.unread;
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
    tauriInvoke('mark_source_read', { sourceId }).catch(console.error);
  }
}

export function hideItem(id: string) {
  const idx = items.findIndex(i => i.id === id);
  if (idx !== -1) {
    items.splice(idx, 1);
    if (IS_TAURI) {
      tauriInvoke('hide_item', { id }).catch(console.error);
    }
  }
}

export async function addSource(
  name: string,
  url: string,
  kind: 'hn' | 'reddit' | 'rss',
  group: string,
): Promise<string> {
  const id = crypto.randomUUID();
  if (IS_TAURI) {
    await tauriInvoke('add_source', {
      source: { id, name, url, kind, group, unread: 0, lastSync: null, enabled: true, itemCount: 0, failureStreak: 0 },
    });
    await reloadSources();
  } else {
    sources.push({
      id, kind, name,
      url,
      host: domainOf(url),
      items: 0, unread: 0,
      lastSync: 'never',
      status: 'stale',
      latencyMs: 0,
      group,
      failureStreak: 0,
    });
  }
  return id;
}

export async function updateSource(
  id: string,
  name: string,
  url: string,
  kind: 'hn' | 'reddit' | 'rss',
  group: string,
): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('update_source', { id, name, url, kind, group });
    await reloadSources();
  } else {
    const s = sources.find(s => s.id === id);
    if (s) {
      s.name = name;
      s.url = url;
      s.kind = kind;
      s.host = domainOf(url);
      s.group = group;
    }
  }
}

export async function removeSource(id: string): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('delete_source', { id });
    for (let i = items.length - 1; i >= 0; i--) {
      if (items[i].src === id) items.splice(i, 1);
    }
    await Promise.all([reloadSources(), reloadGroups()]);
  } else {
    const idx = sources.findIndex(s => s.id === id);
    if (idx !== -1) {
      for (let i = items.length - 1; i >= 0; i--) {
        if (items[i].src === id) items.splice(i, 1);
      }
      sources.splice(idx, 1);
    }
  }
}

export async function syncSource(sourceId: string): Promise<void> {
  if (IS_TAURI) {
    try {
      await tauriInvoke('sync_source', { sourceId });
      await Promise.all([reloadItems(), reloadSources(), reloadGroups()]);
    } catch (e) {
      console.error('[pulse] sync_source failed:', e);
    }
  } else {
    await doSync();
  }
}

export async function clearItems(): Promise<void> {
  if (IS_TAURI) {
    await tauriInvoke('clear_items');
    items.splice(0, items.length);
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
  if (g) g.name = name;
  if (IS_TAURI) {
    tauriInvoke('rename_group', { id, name }).catch(console.error);
  }
}

export async function deleteGroup(id: string): Promise<void> {
  if (id === 'all') return;
  for (const s of sources) { if (s.group === id) s.group = 'all'; }
  if (IS_TAURI) {
    await tauriInvoke('delete_group', { id });
    await reloadGroups();
  } else {
    const idx = groups.findIndex(g => g.id === id);
    if (idx !== -1) groups.splice(idx, 1);
  }
}

// --- Search ---

export async function searchItems(query: string, limit = 100): Promise<FeedItem[]> {
  if (!IS_TAURI) {
    const q = query.toLowerCase();
    return items.filter(i =>
      i.title.toLowerCase().includes(q) ||
      i.body?.toLowerCase().includes(q) ||
      i.tags.some(t => t.toLowerCase().includes(q))
    ).slice(0, limit);
  }
  const bi = await tauriInvoke<BackendItem[]>('search_items', { query, limit });
  return bi.map(adaptItem);
}

// --- AI management ---

export async function reloadAiInfo(): Promise<void> {
  if (!IS_TAURI) return;
  await Promise.all([
    reloadAiStatus().catch(console.error),
    reloadModelsInternal().catch(console.error),
  ]);
}

export async function downloadModel(modelId: string): Promise<void> {
  if (!IS_TAURI) return;
  await tauriInvoke('download_model', { modelId });
  await reloadAiInfo();
}

export async function deleteModel(modelId: string): Promise<void> {
  if (!IS_TAURI) return;
  await tauriInvoke('delete_model', { modelId });
  await reloadAiInfo();
}

export async function retagAll(): Promise<number> {
  if (!IS_TAURI) return 0;
  const count = await tauriInvoke<number>('retag_all');
  await Promise.all([reloadItems(), reloadAiStatus()]);
  return count;
}

export async function activateModel(modelId: string): Promise<void> {
  if (!IS_TAURI) return;
  await tauriInvoke('activate_model', { modelId });
  await reloadAiInfo();
}
