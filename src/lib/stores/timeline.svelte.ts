import { IS_TAURI, tauriInvoke, items, storeReady, adaptItem, reloadDbStats, reloadSources, reloadGroups } from './data.svelte';
import type { BackendItem } from '../types';
import { settings } from '../settings.svelte';
import { logger } from '../logger';


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
export const pageCounts = $state({ total: 0, unread: 0, saved: 0, signal: 0 });

const MAX_CACHED_ITEMS = 500;
const EVICT_COUNT = 100;

// Monotonic request generation. Every filter change bumps it; any in-flight
// get_items_page response whose captured generation is stale is discarded, so a
// slow response can never overwrite a newer filter's list (the "flash then
// reverts to All" bug).
let epoch = 0;

/** Read the current request generation (for callers that refresh the current
 * filter from outside the timeline store, e.g. reloadItems after a sync). */
export function currentEpoch(): number {
  return epoch;
}

function filterArgs(limit: number, cursor?: { publishedAt: number; itemId: string } | null) {
  return {
    groupId: timelineFilter.groupId ?? null,
    feedId: timelineFilter.feedId ?? null,
    tag: timelineFilter.tag ?? null,
    isRead: timelineFilter.isRead,
    isSaved: timelineFilter.isSaved,
    limit,
    cursor: cursor ? { publishedAt: cursor.publishedAt, itemId: cursor.itemId } : null,
  };
}

async function resetAndFetch(): Promise<void> {
  if (!IS_TAURI || storeReady.loading) return;
  const myEpoch = ++epoch;
  loadingMore.cursor = null;
  const page = await tauriInvoke<BackendPage>('get_items_page', filterArgs(100));
  if (myEpoch !== epoch) return; // a newer filter change superseded this response
  items.splice(0, items.length, ...page.items.map(adaptItem));
  loadingMore.cursor = page.nextCursor ?? null;
  if (page.counts) {
    pageCounts.total = page.counts.total;
    pageCounts.unread = page.counts.unread;
    pageCounts.saved = page.counts.saved;
    pageCounts.signal = page.counts.signal;
  }
}

export async function fetchNextPage(): Promise<void> {
  if (!IS_TAURI || !loadingMore.cursor || loadingMore.active || storeReady.loading) return;
  const myEpoch = epoch;
  loadingMore.active = true;
  try {
    const page = await tauriInvoke<BackendPage>('get_items_page', filterArgs(100, loadingMore.cursor));
    if (myEpoch !== epoch) return; // filter changed mid-fetch; discard this page
    items.push(...page.items.map(adaptItem));
    if (items.length > MAX_CACHED_ITEMS) {
      items.splice(0, EVICT_COUNT);
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

export type TimelineFilterPatch = Partial<{
  groupId: string | null; feedId: string | null; tag: string | null;
  isRead: boolean | null; isSaved: boolean | null;
}>;

/** Apply several filter fields atomically and fetch once (avoids the stacked
 * concurrent-fetch race that a sequence of single-field setters produces). */
export async function applyFilter(patch: TimelineFilterPatch): Promise<void> {
  if (storeReady.loading) return;
  Object.assign(timelineFilter, patch);
  await resetAndFetch();
}

export async function setFeedFilter(feedId: string | null): Promise<void> {
  await applyFilter({ feedId, groupId: feedId ? null : timelineFilter.groupId });
}

export async function setGroupFilter(groupId: string | null): Promise<void> {
  await applyFilter({ groupId, feedId: null });
}

export async function setTagFilter(tag: string | null): Promise<void> {
  await applyFilter({ tag });
}

export async function setReadFilter(isRead: boolean | null): Promise<void> {
  await applyFilter({ isRead });
}

export async function setSavedFilter(isSaved: boolean | null): Promise<void> {
  await applyFilter({ isSaved });
}
