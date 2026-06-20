import { IS_TAURI, tauriInvoke, items, storeReady, adaptItem, reloadDbStats, reloadSources, reloadGroups } from './data.svelte';
import { settings } from '../settings.svelte';
import { logger } from '../logger';

interface BackendItem {
  id: string; sourceId: string; sourceName: string; title: string; url: string;
  body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null;
  publishedAt: string; read: boolean; saved: boolean; hidden: boolean;
  score: number | null; n: number; tags: string[]; signal: number;
  ogImage: string | null; note: string | null;
}

interface BackendPage {
  items: BackendItem[];
  nextCursor: { publishedAt: number; itemId: string } | null;
  counts: { total: number; unread: number; saved: number; signal: number };
}

export const timelineFilter = $state<{
  groupId: string | null; feedId: string | null; tag: string | null;
  isRead: boolean | null; isSaved: boolean | null;
}>({ groupId: null, feedId: null, tag: null, isRead: null, isSaved: null });

export const loadingMore = $state({ active: false, cursor: null as { publishedAt: number; itemId: string } | null });
export const hasPrecedingItems = $state({ value: false });
export const pageCounts = $state({ total: 0, unread: 0, saved: 0, signal: 0 });

const MAX_CACHED_ITEMS = 500;
const EVICT_COUNT = 100;

function filterArgs(limit: number, cursor?: { publishedAt: number; itemId: string } | null) {
  return {
    groupId: timelineFilter.groupId ?? null,
    feedId: timelineFilter.feedId ?? null,
    tag: timelineFilter.tag ?? null,
    isRead: timelineFilter.isRead,
    isSaved: timelineFilter.isSaved,
    signalThreshold: settings.confidenceThreshold,
    limit,
    cursor: cursor ? { publishedAt: cursor.publishedAt, itemId: cursor.itemId } : null,
  };
}

async function resetAndFetch(): Promise<void> {
  if (!IS_TAURI || storeReady.loading) return;
  loadingMore.cursor = null;
  const page = await tauriInvoke<BackendPage>('get_items_page', filterArgs(100));
  items.splice(0, items.length, ...page.items.map(adaptItem));
  loadingMore.cursor = page.nextCursor ?? null;
  hasPrecedingItems.value = false;
  if (page.counts) {
    pageCounts.total = page.counts.total;
    pageCounts.unread = page.counts.unread;
    pageCounts.saved = page.counts.saved;
    pageCounts.signal = page.counts.signal;
  }
}

export async function fetchNextPage(): Promise<void> {
  if (!IS_TAURI || !loadingMore.cursor || loadingMore.active || storeReady.loading) return;
  loadingMore.active = true;
  try {
    const page = await tauriInvoke<BackendPage>('get_items_page', filterArgs(100, loadingMore.cursor));
    items.push(...page.items.map(adaptItem));
    if (items.length > MAX_CACHED_ITEMS) {
      items.splice(0, EVICT_COUNT);
      hasPrecedingItems.value = true;
    }
    loadingMore.cursor = page.nextCursor ?? null;
    if (page.counts) {
      pageCounts.total = page.counts.total;
      pageCounts.unread = page.counts.unread;
      pageCounts.saved = page.counts.saved;
      pageCounts.signal = page.counts.signal;
    }
  } catch (e) {
    logger.warn('fetchNextPage failed', e);
  } finally {
    loadingMore.active = false;
  }
}

export async function setFeedFilter(feedId: string | null): Promise<void> {
  if (storeReady.loading) return;
  timelineFilter.feedId = feedId;
  if (feedId) timelineFilter.groupId = null;
  await resetAndFetch();
}

export async function setGroupFilter(groupId: string | null): Promise<void> {
  if (storeReady.loading) return;
  timelineFilter.groupId = groupId;
  timelineFilter.feedId = null;
  await resetAndFetch();
}

export async function setTagFilter(tag: string | null): Promise<void> {
  if (storeReady.loading) return;
  timelineFilter.tag = tag;
  await resetAndFetch();
}

export async function setReadFilter(isRead: boolean | null): Promise<void> {
  if (storeReady.loading) return;
  timelineFilter.isRead = isRead;
  await resetAndFetch();
}

export async function setSavedFilter(isSaved: boolean | null): Promise<void> {
  if (storeReady.loading) return;
  timelineFilter.isSaved = isSaved;
  await resetAndFetch();
}
