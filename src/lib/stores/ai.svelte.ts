import { IS_TAURI, tauriInvoke, items, adaptItem, reloadItems } from './data.svelte';
import { settings } from '../settings.svelte';
import { logger } from '../logger';
import type { AiStatus, ModelInfo, FeedItem } from '../types';

interface BackendAiStatus {
  modelLoaded: boolean; visionLoaded: boolean; fasttextLoaded: boolean; minimlLoaded: boolean;
  modelName: string | null; visionModelName: string | null; fasttextModelName: string | null;
  minimlModelName: string | null; taggingMode: string;
}

interface BackendModelInfo {
  id: string; name: string; description: string; sizeMb: number;
  downloaded: boolean; active: boolean; kind: string;
}

interface BackendAiStats {
  taggedCount: number; avgScore: number; tagCounts: [string, number][]; highSignal: { id: string; sourceId: string; sourceName: string; title: string; url: string; body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null; publishedAt: string; read: boolean; saved: boolean; hidden: boolean; score: number | null; n: number; tags: string[]; signal: number; ogImage: string | null; note: string | null }[];
}

// --- Reactive state ---
export const aiStatus = $state<AiStatus>({
  modelLoaded: false, visionLoaded: false, fasttextLoaded: false, minimlLoaded: false,
  modelName: null, visionModelName: null, fasttextModelName: null, minimlModelName: null,
  taggingMode: 'loading',
});

export const models = $state<ModelInfo[]>([]);
export const taggingProgress = $state({ active: false, tagged: 0, total: 0 });
export const aiStats = $state({ taggedCount: 0, avgScore: 0, tagCounts: [] as [string, number][], highSignal: [] as FeedItem[] });

// --- Internal ---
async function reloadAiStatus(): Promise<void> {
  const s = await tauriInvoke<BackendAiStatus>('get_ai_status');
  aiStatus.modelLoaded = s.modelLoaded;
  aiStatus.visionLoaded = s.visionLoaded;
  aiStatus.fasttextLoaded = s.fasttextLoaded;
  aiStatus.minimlLoaded = s.minimlLoaded;
  aiStatus.modelName = s.modelName;
  aiStatus.visionModelName = s.visionModelName;
  aiStatus.fasttextModelName = s.fasttextModelName;
  aiStatus.minimlModelName = s.minimlModelName;
  aiStatus.taggingMode = s.taggingMode;
}

async function reloadModels(): Promise<void> {
  const ms = await tauriInvoke<BackendModelInfo[]>('list_models');
  models.splice(0, models.length, ...ms.map(m => ({ ...m, kind: m.kind as 'nli' | 'vision' | 'fasttext' | 'miniml' })));
}

export async function reloadAiStats(): Promise<void> {
  if (!IS_TAURI) return;
  const s = await tauriInvoke<BackendAiStats>('get_ai_stats', { signalThreshold: settings.confidenceThreshold });
  aiStats.taggedCount = s.taggedCount;
  aiStats.avgScore = s.avgScore;
  aiStats.tagCounts = s.tagCounts;
  aiStats.highSignal = s.highSignal.map(adaptItem);
}

// --- Public ---
export async function setupTaggingListener(): Promise<() => void> {
  if (!IS_TAURI) return () => {};
  const { listen } = await import('@tauri-apps/api/event');
  return listen<{ tagged: number; total: number; done: boolean }>('ai://tagging-progress', (ev) => {
    const { tagged, total, done } = ev.payload;
    if (done) { taggingProgress.active = false; taggingProgress.tagged = total; taggingProgress.total = total; }
    else { taggingProgress.active = true; taggingProgress.tagged = tagged; taggingProgress.total = total; }
  });
}

export async function reloadAiInfo(): Promise<void> {
  if (!IS_TAURI) return;
  await Promise.all([
    reloadAiStatus().catch(e => logger.warn('reloadAiStatus failed', e)),
    reloadModels().catch(e => logger.warn('reloadModels failed', e)),
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
  await Promise.all([reloadItems(), reloadAiStatus(), reloadAiStats()]);
  return count;
}

export async function activateModel(modelId: string): Promise<void> {
  if (!IS_TAURI) return;
  await tauriInvoke('activate_model', { modelId });
  await reloadAiInfo();
}
