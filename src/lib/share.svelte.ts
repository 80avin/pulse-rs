import { groups, addSource as storeAddSource, syncSource as storeSyncSource, createGroup } from '$lib/store.svelte';

const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

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
    storeSyncSource(newId).catch(console.error);
  } catch (e) {
    console.error('[share] add failed:', e);
  }
}

async function handleIncomingUrl(url: string): Promise<void> {
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
  try {
    const pending = await tauriInvoke<string | null>('get_pending_share');
    if (pending) await handleIncomingUrl(pending);
  } catch {
    /* ignore — app may not be fully initialized */
  }
  const { listen } = await import('@tauri-apps/api/event');
  return listen<{ url: string }>('share://incoming-url', (ev) => {
    handleIncomingUrl(ev.payload.url);
  });
}
