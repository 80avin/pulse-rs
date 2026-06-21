<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources } from '$lib/stores/data.svelte';
  import { aiStatus, models, taggingProgress, downloadModel, deleteModel, activateModel, retagAll, reloadAiInfo, aiStats } from '$lib/stores/ai.svelte';
  import { settings } from '$lib/settings.svelte';
  import { logger } from '$lib/logger';
  import ScoreBar from '$lib/components/ScoreBar.svelte';
  import TagChip from '$lib/components/TagChip.svelte';

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

  let { compact = false, onTagFilter, onItemClick, onSourceFilter }: { compact?: boolean; onTagFilter?: (tag: string) => void; onItemClick?: (id: string, ids: string[]) => void; onSourceFilter?: (sourceId: string) => void } = $props();

  // Use global AI stats from backend instead of deriving from paginated items
  const taggedCount = $derived(aiStats.taggedCount);
  const avgScore = $derived(aiStats.avgScore);
  const tagCounts = $derived(aiStats.tagCounts);
  const highSignal = $derived(aiStats.highSignal);

  let downloadProgress = $state<Record<string, { pct: number; file: string }>>({});
  let downloading = $state<Set<string>>(new Set());
  let retagResult = $state<string | null>(null);

  $effect(() => {
    if (!IS_TAURI) return;
    let unlisten: (() => void) | null = null;
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen<{ modelId: string; file: string; bytesDone: number; bytesTotal: number; done: boolean }>(
        'ai://download-progress',
        (ev) => {
          const { modelId, file, bytesDone, bytesTotal, done } = ev.payload;
          if (done) {
            delete downloadProgress[modelId];
            downloading.delete(modelId);
            reloadAiInfo();
          } else {
            const pct = bytesTotal > 0 ? Math.round((bytesDone / bytesTotal) * 100) : 0;
            downloadProgress[modelId] = { pct, file };
          }
        }
      ).then(u => { unlisten = u; });
    });
    return () => { unlisten?.(); };
  });

  async function handleDownload(modelId: string) {
    downloading.add(modelId);
    downloadProgress[modelId] = { pct: 0, file: 'starting…' };
    try {
      await downloadModel(modelId);
    } catch (e) {
      logger.error('model download failed', { modelId, error: String(e) });
      delete downloadProgress[modelId];
      downloading.delete(modelId);
    }
  }

  async function handleDelete(modelId: string) {
    try { await deleteModel(modelId); } catch (e) { logger.error('model delete failed', { modelId, error: String(e) }); }
  }

  async function handleActivate(modelId: string) {
    try { await activateModel(modelId); } catch (e) { logger.error('model activate failed', { modelId, error: String(e) }); }
  }

  async function handleRetag() {
    if (taggingProgress.active) return;
    if (!settings.aiTagging) {
      retagResult = 'ai tagging is disabled — enable it in settings';
      return;
    }
    retagResult = null;
    try {
      const count = await retagAll();
      retagResult = count > 0 ? `${count} tags applied` : 'done — no new tags (check confidence threshold)';
    } catch {
      retagResult = 'error — check console';
    }
  }

  const gap = $derived(compact ? '10px' : '12px');
  const sectionPad = $derived(compact ? '10px' : '12px');
  const titleSize = $derived(compact ? '9px' : '9px');
</script>

<div class="flex flex-col" style="gap:{gap};">
  <!-- Experimental notice -->
  <div class="p-2.5 px-3 bg-transparent border border-amber rounded flex items-start gap-2">
    <span class="text-amber uppercase shrink-0 mt-px text-[10px] leading-none font-mono tracking-[0.5px] p-[2px_5px] border border-amber rounded-sm">experimental</span>
    <span class="text-ink-2 text-[10px] leading-normal font-mono">Tags may be inaccurate. Raise the confidence threshold in Settings or disable AI tagging if results look wrong.</span>
  </div>

  <!-- Model status card -->
  <div class="bg-bg-1 border border-bd-0 rounded" style="padding:{sectionPad};">
    <div class="text-ink-3 uppercase mb-2.5 font-mono tracking-[0.6px]" style="font:{titleSize}/1;">model status</div>

    <!-- FastText row -->
    <div class="flex items-center gap-2.5 mb-1.5">
      <span class="w-2 h-2 rounded-full shrink-0" style="background:{aiStatus.fasttextLoaded ? T.cyan : T.ink3};"></span>
      <span class="text-ink-0 flex-1 text-[11px] leading-none font-mono">
        {aiStatus.fasttextLoaded ? (aiStatus.fasttextModelName ?? 'fasttext') : 'fasttext · not loaded'}
      </span>
      {#if aiStatus.fasttextLoaded}<span class="text-cyan text-[10px] leading-none font-mono p-[1px_5px] border border-cyan rounded-sm">text</span>{/if}
    </div>

    <!-- MiniLM row -->
    <div class="flex items-center gap-2.5 mb-1.5">
      <span class="w-2 h-2 rounded-full shrink-0" style="background:{aiStatus.minimlLoaded ? T.cyan : T.ink3};"></span>
      <span class="text-ink-0 flex-1 text-[11px] leading-none font-mono">
        {aiStatus.minimlLoaded ? (aiStatus.minimlModelName ?? 'miniml') : 'miniml · not loaded'}
      </span>
      {#if aiStatus.minimlLoaded}<span class="text-cyan text-[10px] leading-none font-mono p-[1px_5px] border border-cyan rounded-sm">semantic</span>{/if}
    </div>

    <!-- Vision model row -->
    <div class="flex items-center gap-2.5 mb-3">
      <span class="w-2 h-2 rounded-full shrink-0" style="background:{aiStatus.visionLoaded ? T.cyan : T.ink3};"></span>
      <span class="text-ink-0 flex-1 text-[11px] leading-none font-mono">
        {aiStatus.visionLoaded ? (aiStatus.visionModelName ?? 'clip vision') : 'clip vision · not loaded'}
      </span>
      {#if aiStatus.visionLoaded}<span class="text-cyan text-[10px] leading-none font-mono p-[1px_5px] border border-cyan rounded-sm">vision</span>{/if}
    </div>

    <!-- Stats grid -->
    <div class="grid grid-cols-3 gap-2">
      {#each [
        { label: 'tagged',    val: String(taggedCount),                       color: T.cyan  },
        { label: 'avg score', val: taggedCount > 0 ? avgScore.toFixed(2) : '—',   color: T.amber },
        { label: 'tags',      val: String(tagCounts.length),                         color: T.ink1  },
      ] as stat}
        <div class="p-2 bg-bg-0 border border-bd-0 rounded text-center">
          <div class="tabular-nums font-mono" style="font-size:{compact ? '14px' : '16px'};line-height:1;color:{stat.color};">{stat.val}</div>
          <div class="mt-1.5 text-ink-3 text-[10px] leading-none font-mono">{stat.label}</div>
        </div>
      {/each}
    </div>

    <!-- Mode label -->
    <div class="mt-2 text-ink-3 text-[10px] leading-none font-mono">
      mode: <span style="color:{aiStatus.taggingMode === 'loading' ? T.ink3 : aiStatus.taggingMode === 'none' ? T.amber : T.cyan};">{aiStatus.taggingMode}</span>
    </div>

    <!-- Re-tag all + live progress -->
    <div class="mt-2.5">
      <div class="flex items-center gap-2">
        <button
          onclick={handleRetag}
          disabled={taggingProgress.active || !IS_TAURI || !settings.aiTagging}
          class="flex-1 p-2 border rounded text-[10px] leading-none font-mono" style="background:{taggingProgress.active ? T.bg0 : T.bg2};border-color:{taggingProgress.active ? T.amber : T.bd1};color:{taggingProgress.active ? T.amber : !settings.aiTagging ? T.ink3 : T.ink0};cursor:{taggingProgress.active || !settings.aiTagging ? 'default' : 'pointer'};"
        >
          {taggingProgress.active ? `tagging ${taggingProgress.tagged} / ${taggingProgress.total}…` : 're-tag all items'}
        </button>
        {#if retagResult && !taggingProgress.active}
          <span class="text-cyan text-[10px] leading-none font-mono">{retagResult}</span>
        {/if}
      </div>
      {#if taggingProgress.active && taggingProgress.total > 0}
        {@const pct = Math.round((taggingProgress.tagged / taggingProgress.total) * 100)}
        <div class="mt-1.5">
          <div class="h-0.5 bg-bg-0 rounded-px overflow-hidden">
            <div class="h-full bg-amber rounded-px" style="width:{pct}%;transition:width 0.15s;"></div>
          </div>
          <div class="mt-1 text-ink-3 text-[10px] leading-none font-mono">{pct}% complete</div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Model download section — vision + miniml (fasttext is bundled, no download needed) -->
  <div class="bg-bg-1 border border-bd-0 rounded" style="padding:{sectionPad};">
    <div class="text-ink-3 uppercase mb-2.5 font-mono tracking-[0.6px]" style="font:{titleSize}/1;">available models</div>
    <div class="flex flex-col gap-2.5">
      {#each models.filter(m => m.kind === 'vision' || m.kind === 'miniml') as m}
        {@const prog = downloadProgress[m.id]}
        {@const isDownloading = downloading.has(m.id)}
        <div class="p-2.5 bg-bg-0 border rounded" style="border-color:{m.active ? T.cyan : T.bd0};">
          <div class="flex items-center gap-2 mb-1">
            <span class="text-ink-0 flex-1 text-[11px] leading-none font-mono">{m.name}</span>
            {#if m.active}
              <span class="text-cyan text-[10px] leading-none font-mono p-[2px_6px] border border-cyan rounded-sm">active</span>
            {:else if m.downloaded}
              <button
                onclick={() => handleActivate(m.id)}
                class="text-amber bg-transparent border border-amber rounded-sm cursor-pointer text-[10px] leading-none font-mono p-[2px_6px]"
              >activate</button>
              <button
                onclick={() => handleDelete(m.id)}
                class="text-red bg-transparent border border-red rounded-sm cursor-pointer text-[10px] leading-none font-mono p-[2px_6px]"
              >remove</button>
            {:else if isDownloading}
              <span class="text-amber text-[10px] leading-none font-mono">downloading</span>
            {:else}
              <button
                onclick={() => handleDownload(m.id)}
                class="text-cyan bg-transparent border border-cyan rounded-sm cursor-pointer text-[10px] leading-none font-mono p-[2px_6px]"
              >download</button>
            {/if}
          </div>
          <div class="text-ink-3 mb-1 text-[10px] leading-[1.4] font-mono">{m.description}</div>
          <div class="text-ink-2 text-[10px] leading-none font-mono">{m.sizeMb} MB · {m.kind === 'miniml' ? 'semantic tagger' : 'vision tagger'}</div>
          {#if prog}
            <div class="mt-2">
              <div class="text-ink-3 mb-1 text-[10px] leading-none font-mono">{prog.file} · {prog.pct}%</div>
              <div class="h-[3px] bg-bg-1 rounded-sm overflow-hidden">
                <div class="h-full bg-cyan rounded-sm" style="width:{prog.pct}%;transition:width 0.2s;"></div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
      {#if models.filter(m => m.kind === 'vision' || m.kind === 'miniml').length === 0}
        <div class="text-ink-3 text-center py-3 text-[10px] leading-[1.4] font-mono">
          {IS_TAURI ? 'no downloadable models available' : 'models shown in Tauri app only'}
        </div>
      {/if}
    </div>
  </div>

  <!-- Tag distribution -->
  {#if tagCounts.length > 0}
    <div class="bg-bg-1 border border-bd-0 rounded" style="padding:{sectionPad};">
      <div class="text-ink-3 uppercase mb-2.5 font-mono tracking-[0.6px]" style="font:{titleSize}/1;">tag distribution</div>
      <div class="flex flex-col gap-1.75">
        {#each tagCounts as [tag, count]}
          {@const maxCount = tagCounts[0][1]}
          {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
          <button
            onclick={() => onTagFilter?.(tag)}
            class="flex items-center gap-2 bg-transparent border-none p-0 w-full" style="cursor:{onTagFilter ? 'pointer' : 'default'};"
            title={onTagFilter ? `filter by ${tag}` : undefined}
          >
            <span class="w-[76px] shrink-0 truncate text-left text-[10px] leading-none font-mono" style="color:{tc.fg};">{tag}</span>
            <div class="flex-1 h-[3px] bg-bg-0 rounded-sm overflow-hidden">
              <div class="h-full rounded-sm" style="width:{(count / maxCount) * 100}%;background:{tc.fg};"></div>
            </div>
            <span class="text-ink-2 tabular-nums min-w-4.5 text-right text-[10px] leading-none font-mono">{count}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- High signal items (hidden in compact mode to save space) -->
  {#if !compact && highSignal.some(i => i.aiScore > 0)}
    <div class="bg-bg-1 border border-bd-0 rounded" style="padding:{sectionPad};">
      <div class="text-ink-3 uppercase mb-2.5 font-mono tracking-[0.6px]" style="font:{titleSize}/1;">highest signal</div>
      <div class="flex flex-col gap-0">
        {#each highSignal.filter(i => i.aiScore > 0) as item, i}
          {@const source = sources.find(s => s.id === item.src)}
          {@const signalIds = highSignal.filter(i => i.aiScore > 0).map(i => i.id)}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <div
            role="button"
            tabindex="0"
            onclick={() => onItemClick?.(item.id, signalIds)}
            onkeydown={(e) => { if (e.key === 'Enter') onItemClick?.(item.id, signalIds); }}
            class="cursor-pointer" style="{i > 0 ? `padding-top:10px;margin-top:10px;border-top:1px solid ${T.bd0};` : ''}"
          >
            <div class="flex items-center gap-1.5 mb-1">
              <ScoreBar value={item.aiScore} w={28} />
              <span class="text-amber tabular-nums text-[10px] leading-none font-mono">{item.aiScore.toFixed(2)}</span>
              {#if source}
                <button
                  onclick={(e) => { e.stopPropagation(); onSourceFilter?.(source.id); }}
                  class="text-ink-3 bg-transparent border-none cursor-pointer p-0 text-[10px] leading-none font-mono"
                  title="Filter by {source.name}"
                >
                  · {source.name}
                </button>
              {/if}
            </div>
            <div class="text-ink-0 overflow-hidden text-[12px] leading-[1.3] font-mono" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;">{item.title}</div>
            {#if item.tags.length > 0}
              <div class="mt-1.5 flex flex-wrap gap-1">
                {#each item.tags.slice(0, 3) as tag}<TagChip {tag} size={9} onclick={() => onTagFilter?.(tag)} />{/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
