<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources, aiStatus, models, taggingProgress, downloadModel, deleteModel, activateModel, retagAll, reloadAiInfo } from '$lib/store.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import ScoreBar from '$lib/components/ScoreBar.svelte';
  import TagChip from '$lib/components/TagChip.svelte';

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;

  let { compact = false, onTagFilter }: { compact?: boolean; onTagFilter?: (tag: string) => void } = $props();

  const taggedItems = $derived(items.filter(i => i.tags.length > 0));

  const avgScore = $derived.by(() => {
    if (!taggedItems.length) return 0;
    return taggedItems.reduce((s, i) => s + i.aiScore, 0) / taggedItems.length;
  });

  const tagCounts = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const item of taggedItems) {
      for (const tag of item.tags) counts[tag] = (counts[tag] ?? 0) + 1;
    }
    return Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 10);
  });

  const highSignal = $derived(
    [...items].sort((a, b) => b.aiScore - a.aiScore).slice(0, 5)
  );

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
      console.error('[pulse] download failed:', e);
      delete downloadProgress[modelId];
      downloading.delete(modelId);
    }
  }

  async function handleDelete(modelId: string) {
    try { await deleteModel(modelId); } catch (e) { console.error('[pulse] delete failed:', e); }
  }

  async function handleActivate(modelId: string) {
    try { await activateModel(modelId); } catch (e) { console.error('[pulse] activate failed:', e); }
  }

  async function handleRetag() {
    if (taggingProgress.active) return;
    retagResult = null;
    try {
      const count = await retagAll();
      retagResult = `${count} tags applied`;
    } catch {
      retagResult = 'error — check console';
    }
  }

  const gap = $derived(compact ? '10px' : '12px');
  const sectionPad = $derived(compact ? '10px' : '12px');
  const titleSize = $derived(compact ? '9px' : '9px');
</script>

<div style="display:flex;flex-direction:column;gap:{gap};">
  <!-- Model status card -->
  <div style="padding:{sectionPad};background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
    <div style="font:{titleSize}/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">model status</div>

    <!-- FastText row -->
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:6px;">
      <span style="width:8px;height:8px;border-radius:50%;background:{aiStatus.fasttextLoaded ? T.cyan : T.ink3};flex-shrink:0;"></span>
      <span style="font:11px/1 {T.mono};color:{T.ink0};flex:1;">
        {aiStatus.fasttextLoaded ? (aiStatus.fasttextModelName ?? 'fasttext') : 'fasttext · not loaded'}
      </span>
      {#if aiStatus.fasttextLoaded}<span style="font:9px/1 {T.mono};color:{T.cyan};padding:1px 5px;border:1px solid {T.cyan};border-radius:2px;">text</span>{/if}
    </div>

    <!-- MiniLM row -->
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:6px;">
      <span style="width:8px;height:8px;border-radius:50%;background:{aiStatus.minimlLoaded ? T.cyan : T.ink3};flex-shrink:0;"></span>
      <span style="font:11px/1 {T.mono};color:{T.ink0};flex:1;">
        {aiStatus.minimlLoaded ? (aiStatus.minimlModelName ?? 'miniml') : 'miniml · not loaded'}
      </span>
      {#if aiStatus.minimlLoaded}<span style="font:9px/1 {T.mono};color:{T.cyan};padding:1px 5px;border:1px solid {T.cyan};border-radius:2px;">semantic</span>{/if}
    </div>

    <!-- Vision model row -->
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:12px;">
      <span style="width:8px;height:8px;border-radius:50%;background:{aiStatus.visionLoaded ? T.cyan : T.ink3};flex-shrink:0;"></span>
      <span style="font:11px/1 {T.mono};color:{T.ink0};flex:1;">
        {aiStatus.visionLoaded ? (aiStatus.visionModelName ?? 'clip vision') : 'clip vision · not loaded'}
      </span>
      {#if aiStatus.visionLoaded}<span style="font:9px/1 {T.mono};color:{T.cyan};padding:1px 5px;border:1px solid {T.cyan};border-radius:2px;">vision</span>{/if}
    </div>

    <!-- Stats grid -->
    <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;">
      {#each [
        { label: 'tagged',    val: String(taggedItems.length),                       color: T.cyan  },
        { label: 'avg score', val: taggedItems.length ? avgScore.toFixed(2) : '—',   color: T.amber },
        { label: 'tags',      val: String(tagCounts.length),                         color: T.ink1  },
      ] as stat}
        <div style="padding:8px;background:{T.bg0};border:1px solid {T.bd0};border-radius:3px;text-align:center;">
          <div style="font:{compact ? '14px' : '16px'}/1 {T.mono};color:{stat.color};font-variant-numeric:tabular-nums;">{stat.val}</div>
          <div style="margin-top:5px;font:9px/1 {T.mono};color:{T.ink3};">{stat.label}</div>
        </div>
      {/each}
    </div>

    <!-- Mode label -->
    <div style="margin-top:8px;font:9px/1 {T.mono};color:{T.ink3};">
      mode: <span style="color:{aiStatus.taggingMode === 'loading' ? T.ink3 : aiStatus.taggingMode === 'none' ? T.amber : T.cyan};">{aiStatus.taggingMode}</span>
    </div>

    <!-- Re-tag all + live progress -->
    <div style="margin-top:10px;">
      <div style="display:flex;align-items:center;gap:8px;">
        <button
          onclick={handleRetag}
          disabled={taggingProgress.active || !IS_TAURI}
          style="flex:1;padding:8px;background:{taggingProgress.active ? T.bg0 : T.bg2};border:1px solid {taggingProgress.active ? T.amber : T.bd1};border-radius:3px;font:10px/1 {T.mono};color:{taggingProgress.active ? T.amber : T.ink0};cursor:{taggingProgress.active ? 'default' : 'pointer'};"
        >
          {taggingProgress.active ? `tagging ${taggingProgress.tagged} / ${taggingProgress.total}…` : 're-tag all items'}
        </button>
        {#if retagResult && !taggingProgress.active}
          <span style="font:10px/1 {T.mono};color:{T.cyan};">{retagResult}</span>
        {/if}
      </div>
      {#if taggingProgress.active && taggingProgress.total > 0}
        {@const pct = Math.round((taggingProgress.tagged / taggingProgress.total) * 100)}
        <div style="margin-top:6px;">
          <div style="height:2px;background:{T.bg0};border-radius:1px;overflow:hidden;">
            <div style="height:100%;width:{pct}%;background:{T.amber};border-radius:1px;transition:width 0.15s;"></div>
          </div>
          <div style="margin-top:4px;font:9px/1 {T.mono};color:{T.ink3};">{pct}% complete</div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Model download section — vision + miniml (fasttext is bundled, no download needed) -->
  <div style="padding:{sectionPad};background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
    <div style="font:{titleSize}/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">available models</div>
    <div style="display:flex;flex-direction:column;gap:10px;">
      {#each models.filter(m => m.kind === 'vision' || m.kind === 'miniml') as m}
        {@const prog = downloadProgress[m.id]}
        {@const isDownloading = downloading.has(m.id)}
        <div style="padding:10px;background:{T.bg0};border:1px solid {m.active ? T.cyan : T.bd0};border-radius:3px;">
          <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;">
            <span style="font:11px/1 {T.mono};color:{T.ink0};flex:1;">{m.name}</span>
            {#if m.active}
              <span style="font:9px/1 {T.mono};color:{T.cyan};padding:2px 6px;border:1px solid {T.cyan};border-radius:2px;">active</span>
            {:else if m.downloaded}
              <button
                onclick={() => handleActivate(m.id)}
                style="font:9px/1 {T.mono};color:{T.amber};background:transparent;border:1px solid {T.amber};border-radius:2px;padding:2px 6px;cursor:pointer;"
              >activate</button>
              <button
                onclick={() => handleDelete(m.id)}
                style="font:9px/1 {T.mono};color:{T.red};background:transparent;border:1px solid {T.red};border-radius:2px;padding:2px 6px;cursor:pointer;"
              >remove</button>
            {:else if isDownloading}
              <span style="font:9px/1 {T.mono};color:{T.amber};">downloading</span>
            {:else}
              <button
                onclick={() => handleDownload(m.id)}
                style="font:9px/1 {T.mono};color:{T.cyan};background:transparent;border:1px solid {T.cyan};border-radius:2px;padding:2px 6px;cursor:pointer;"
              >download</button>
            {/if}
          </div>
          <div style="font:10px/1.4 {T.mono};color:{T.ink3};margin-bottom:4px;">{m.description}</div>
          <div style="font:10px/1 {T.mono};color:{T.ink2};">{m.sizeMb} MB · {m.kind === 'miniml' ? 'semantic tagger' : 'vision tagger'}</div>
          {#if prog}
            <div style="margin-top:8px;">
              <div style="font:9px/1 {T.mono};color:{T.ink3};margin-bottom:4px;">{prog.file} · {prog.pct}%</div>
              <div style="height:3px;background:{T.bg1};border-radius:2px;overflow:hidden;">
                <div style="height:100%;width:{prog.pct}%;background:{T.cyan};border-radius:2px;transition:width 0.2s;"></div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
      {#if models.filter(m => m.kind === 'vision' || m.kind === 'miniml').length === 0}
        <div style="font:10px/1.4 {T.mono};color:{T.ink3};text-align:center;padding:12px 0;">
          {IS_TAURI ? 'no downloadable models available' : 'models shown in Tauri app only'}
        </div>
      {/if}
    </div>
  </div>

  <!-- Tag distribution -->
  {#if tagCounts.length > 0}
    <div style="padding:{sectionPad};background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
      <div style="font:{titleSize}/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">tag distribution</div>
      <div style="display:flex;flex-direction:column;gap:7px;">
        {#each tagCounts as [tag, count]}
          {@const maxCount = tagCounts[0][1]}
          {@const tc = TAG_COLORS[tag] ?? { fg: T.ink2, bg: 'transparent', bd: T.bd1 }}
          <button
            onclick={() => onTagFilter?.(tag)}
            style="display:flex;align-items:center;gap:8px;background:transparent;border:none;padding:0;cursor:{onTagFilter ? 'pointer' : 'default'};width:100%;"
            title={onTagFilter ? `filter by ${tag}` : undefined}
          >
            <span style="width:76px;flex-shrink:0;font:10px/1 {T.mono};color:{tc.fg};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;text-align:left;">{tag}</span>
            <div style="flex:1;height:3px;background:{T.bg0};border-radius:2px;overflow:hidden;">
              <div style="height:100%;width:{(count / maxCount) * 100}%;background:{tc.fg};border-radius:2px;"></div>
            </div>
            <span style="font:10px/1 {T.mono};color:{T.ink2};font-variant-numeric:tabular-nums;min-width:18px;text-align:right;">{count}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- High signal items (hidden in compact mode to save space) -->
  {#if !compact && highSignal.some(i => i.aiScore > 0)}
    <div style="padding:{sectionPad};background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
      <div style="font:{titleSize}/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">highest signal</div>
      <div style="display:flex;flex-direction:column;gap:0;">
        {#each highSignal.filter(i => i.aiScore > 0) as item, i}
          {@const source = sources.find(s => s.id === item.src)}
          <div style="{i > 0 ? `padding-top:10px;margin-top:10px;border-top:1px solid ${T.bd0};` : ''}">
            <div style="display:flex;align-items:center;gap:6px;margin-bottom:4px;">
              <ScoreBar value={item.aiScore} w={28} />
              <span style="font:10px/1 {T.mono};color:{T.amber};font-variant-numeric:tabular-nums;">{item.aiScore.toFixed(2)}</span>
              {#if source}<span style="font:10px/1 {T.mono};color:{T.ink3};">· {source.name}</span>{/if}
            </div>
            <div style="font:12px/1.3 {T.mono};color:{T.ink0};overflow:hidden;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;">{item.title}</div>
            {#if item.tags.length > 0}
              <div style="margin-top:5px;display:flex;flex-wrap:wrap;gap:4px;">
                {#each item.tags.slice(0, 3) as tag}<TagChip {tag} size={9} />{/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
