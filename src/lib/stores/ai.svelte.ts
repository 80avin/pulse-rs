import { IS_TAURI, tauriInvoke } from './data.svelte';
import { logger } from '../logger';

// Tag distribution for the tag filter (from the rule-engine `ai_tags` table).
export const tagStats = $state({ taggedCount: 0, tagCounts: [] as [string, number][] });

export async function reloadTagStats(): Promise<void> {
  if (!IS_TAURI) return;
  try {
    const s = await tauriInvoke<{ taggedCount: number; tagCounts: [string, number][] }>('get_tag_stats');
    tagStats.taggedCount = s.taggedCount;
    tagStats.tagCounts = s.tagCounts;
  } catch (e) {
    logger.warn('reloadTagStats failed', e);
  }
}
