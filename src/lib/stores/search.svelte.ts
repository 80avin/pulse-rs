import { IS_TAURI, tauriInvoke, items, adaptItem } from './data.svelte';
import type { FeedItem } from '../types';

interface BackendItem {
  id: string; sourceId: string; sourceName: string; title: string; url: string;
  body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null;
  publishedAt: string; read: boolean; saved: boolean; hidden: boolean;
  score: number | null; n: number; tags: string[]; signal: number;
  ogImage: string | null; note: string | null; userTags: string[];
}

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
