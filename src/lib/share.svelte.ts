import { groups, addSource as storeAddSource, syncSource as storeSyncSource, createGroup } from '$lib/stores/data.svelte';
import { logger } from '$lib/logger';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

export interface FeedLinkDto {
  url: string;
  title: string | null;
}

export interface FeedCandidateDto {
  feedUrl: string;
  kind: string;
  name: string;
  isDirectFeed: boolean;
  isHn: boolean;
  noFeedFound: boolean;
  candidates: FeedLinkDto[];
}

export const shareSheet = $state({
  candidate: null as FeedCandidateDto | null,
  loading: false,
  error: null as string | null,
  name: '',
  feedUrl: '',
  kind: 'rss' as 'reddit' | 'hn' | 'rss',
  group: 'all',
  newGroupName: '',
});

export function dismissShare() {
  shareSheet.candidate = null;
  shareSheet.loading = false;
  shareSheet.error = null;
  shareSheet.newGroupName = '';
}

export async function confirmShare(): Promise<void> {
  let { name, feedUrl, kind, group, newGroupName } = shareSheet;
  if (group === '__new__') {
    if (!newGroupName.trim()) return;
    await createGroup(newGroupName.trim());
    group = newGroupName.trim().toLowerCase().replace(/\s+/g, '-').replace(/[^a-z0-9-]/g, '');
  }
  dismissShare();
  try {
    const newId = await storeAddSource(name || feedUrl, feedUrl, kind, group);
    storeSyncSource(newId).catch(e => logger.warn('sync after share-add failed', e));
  } catch (e) {
    logger.error('share: add source failed', e);
  }
}

let lastHandledUrl: string | null = null;
let lastHandledAt = 0;

async function handleIncomingUrl(url: string): Promise<void> {
  // Dedup: on cold start the JNI both buffers the URL and emits the event, so
  // the live listener and the get_pending_share drain can deliver the SAME URL
  // back-to-back. A second identical share within a few seconds is almost
  // certainly that race, not a real double-share — skip it.
  if (lastHandledUrl === url && Date.now() - lastHandledAt < 5000) return;
  lastHandledUrl = url;
  lastHandledAt = Date.now();

  shareSheet.loading = true;
  shareSheet.error = null;
  shareSheet.candidate = {
    feedUrl: url,
    kind: 'rss',
    name: '',
    isDirectFeed: false,
    isHn: false,
    noFeedFound: false,
    candidates: [],
  };
  shareSheet.feedUrl = url;
  shareSheet.name = '';
  shareSheet.kind = 'rss';
  shareSheet.group = groups[0]?.id ?? 'all';
  try {
    const result = await tauriInvoke<FeedCandidateDto>('detect_feed', { url });
    shareSheet.candidate = result;
    shareSheet.feedUrl = result.feedUrl;
    shareSheet.name = result.name;
    shareSheet.kind = result.kind as 'reddit' | 'hn' | 'rss';
  } catch (e) {
    shareSheet.error = String(e);
  } finally {
    shareSheet.loading = false;
  }
}

export async function setupShareListener(): Promise<() => void> {
  if (!IS_TAURI) return () => {};
  // Register the live listener BEFORE draining the cold-start buffer, so no
  // share can slip between the drain and listener registration.
  const { listen } = await import('@tauri-apps/api/event');
  const unlisten = await listen<{ url: string }>('share://incoming-url', (ev) => {
    handleIncomingUrl(ev.payload.url);
  });
  try {
    const pending = await tauriInvoke<string | null>('get_pending_share');
    if (pending) await handleIncomingUrl(pending);
  } catch {
    /* ignore — app may not be fully initialized */
  }
  return unlisten;
}
