<script lang="ts">
  import { T, TAG_COLORS } from '$lib/tokens';
  import { items, sources, markRead, toggleSaved, hideItem, aiStatus } from '$lib/store.svelte';
  import { settings } from '$lib/settings.svelte';
  import { openExternal, sanitizeHtml, TAG_EVIDENCE } from '$lib/utils';
  import TagChip from '$lib/components/TagChip.svelte';
  import ScoreBar from '$lib/components/ScoreBar.svelte';
  import SourceGlyph from '$lib/components/SourceGlyph.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import Icon from '$lib/components/Icon.svelte';

  let { itemId, allIds, onBack, onNavigate }: {
    itemId: string;
    allIds: string[];
    onBack: () => void;
    onNavigate: (id: string) => void;
  } = $props();

  const item   = $derived(items.find(i => i.id === itemId));
  const source = $derived(item ? sources.find(s => s.id === item.src) : undefined);
  const idx    = $derived(allIds.indexOf(itemId));
  const hasPrev = $derived(idx > 0);
  const hasNext = $derived(idx < allIds.length - 1);

  let popoverTag = $state<string | null>(null);

  $effect(() => { if (itemId && settings.markReadOn === 'open') markRead(itemId); });

  function goNext() { if (hasNext) onNavigate(allIds[idx + 1]); }
  function goPrev() { if (hasPrev) onNavigate(allIds[idx - 1]); }

  function handleKey(e: KeyboardEvent) {
    // When popover is open, only handle Escape
    if (popoverTag) {
      if (e.key === 'Escape') { e.preventDefault(); popoverTag = null; }
      return;
    }
    switch (e.key) {
      case 'j': case 'ArrowDown': goNext(); break;
      case 'k': case 'ArrowUp':   goPrev(); break;
      case 'm': if (item) markRead(item.id, !item.read); break;
      case 's': if (item) toggleSaved(item.id); break;
      case 'o': if (item?.url || item?.domain) openExternal(item.url ?? `https://${item.domain}`); break;
      case 'Escape': onBack(); break;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

{#if item}
  <div style="position:relative;display:flex;flex-direction:column;height:100%;background:{T.bg0};color:{T.ink0};">

    <!-- Top bar -->
    <div style="height:44px;display:flex;align-items:center;padding:0 8px;border-bottom:1px solid {T.bd0};background:{T.bg1};flex-shrink:0;gap:6px;">
      <button
        onclick={onBack}
        style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;">
        <Icon name="arrow-l" size={18} color={T.ink1} />
      </button>
      <span style="font:11px/1 {T.mono};color:{T.ink2};flex:1;">
        reader · {idx + 1}<span style="color:{T.ink3};">/{allIds.length}</span>
      </span>
      <button onclick={goPrev} disabled={!hasPrev} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;opacity:{hasPrev ? 1 : 0.3};">
        <Icon name="arrow-up" size={18} color={T.ink1} />
      </button>
      <button onclick={goNext} disabled={!hasNext} style="width:34px;height:34px;display:inline-flex;align-items:center;justify-content:center;background:transparent;border:none;cursor:pointer;border-radius:4px;opacity:{hasNext ? 1 : 0.3};">
        <Icon name="arrow-dn" size={18} color={T.ink1} />
      </button>
    </div>

    <!-- Scrollable body -->
    <div style="flex:1;overflow-y:auto;">
      <!-- Header -->
      <div style="padding:12px 14px;border-bottom:1px solid {T.bd0};">
        <div style="display:flex;align-items:center;gap:8px;font:10px/1 {T.mono};color:{T.ink2};">
          {#if source}
            <SourceGlyph kind={source.kind} />
            <span style="color:{T.ink1};">{source.name}</span>
            <span style="color:{T.ink3};">·</span>
          {/if}
          <span>{item.author}</span>
          <span style="color:{T.ink3};">·</span>
          <span>{item.age}</span>
          {#if item.score > 0}
            <span style="color:{T.ink3};">·</span>
            <span style="color:{T.amber};">▲{item.score}</span>
          {/if}
          <span style="flex:1;"></span>
          {#if item.n > 0}<span style="color:{T.ink3};">{item.n}c</span>{/if}
        </div>

        <h1 style="margin:8px 0 0;font:600 18px/1.25 {T.sans};color:{T.ink0};letter-spacing:-0.2px;">{item.title}</h1>

        {#if item.url}
          <button
            onclick={() => openExternal(item.url!)}
            style="margin-top:7px;display:inline-flex;align-items:center;gap:6px;background:transparent;border:none;cursor:pointer;padding:0;font:10px/1 {T.mono};color:{T.ink2};"
          >
            <Icon name="ext" size={11} color={T.ink2} />
            <span style="text-decoration:underline;text-underline-offset:2px;text-decoration-color:{T.bd2};">{new URL(item.url).hostname.replace(/^www\./, '')}</span>
          </button>
        {/if}
        {#if item.externalUrl}
          <button
            onclick={() => openExternal(item.externalUrl!)}
            style="margin-top:3px;display:block;font:10px/1.4 {T.mono};color:{T.cyan};background:transparent;border:none;cursor:pointer;padding:0;text-align:left;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;"
          >
            <Icon name="ext" size={10} color={T.cyan} />
            <span style="margin-left:3px;">{item.externalUrl}</span>
          </button>
        {/if}

        <div style="margin-top:9px;display:flex;align-items:center;gap:5px;flex-wrap:wrap;">
          {#each item.tags as tag}
            <TagChip {tag} size={10} onclick={() => { popoverTag = tag; }} />
          {/each}
          <span style="flex:1;"></span>
          <span style="font:10px/1 {T.mono};color:{T.ink3};margin-right:6px;">signal</span>
          <ScoreBar value={item.aiScore} w={28} />
        </div>
      </div>

      <!-- Body -->
      <div style="padding:16px 14px 32px;font:14px/1.6 {T.sans};color:{T.ink0};" class="item-body">
        {#if item.bodyHtml}
          {@html sanitizeHtml(item.bodyHtml)}
        {:else if item.body}
          <p style="margin:0;white-space:pre-line;">{item.body}</p>
        {/if}
        {#if item.url}
          <div style="margin-top:20px;padding-top:14px;border-top:1px solid {T.bd0};display:flex;gap:8px;flex-wrap:wrap;">
            <button
              onclick={() => openExternal(item.url!)}
              style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 14px;cursor:pointer;font:12px/1 {T.mono};color:{T.cyan};"
            >
              <Icon name="ext" size={13} color={T.cyan} />
              <span>open post</span>
            </button>
            {#if item.externalUrl}
              <button
                onclick={() => openExternal(item.externalUrl!)}
                style="display:inline-flex;align-items:center;gap:8px;background:{T.bg1};border:1px solid {T.bd1};border-radius:3px;padding:10px 14px;cursor:pointer;font:12px/1 {T.mono};color:{T.ink1};"
              >
                <Icon name="ext" size={13} color={T.ink2} />
                <span>open link</span>
              </button>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Action bar -->
    <div style="display:flex;border-top:1px solid {T.bd1};background:{T.bg1};flex-shrink:0;">
      {#each [
        { icon: 'check',    label: 'read', key: 'm', active: item.read,  color: T.green, action: () => markRead(item!.id, !item!.read) },
        { icon: 'bookmark', label: 'save', key: 's', active: item.saved, color: T.amber, action: () => toggleSaved(item!.id) },
        { icon: 'ext',      label: 'open', key: 'o', active: false,      color: T.cyan,  action: () => item?.url && openExternal(item.url) },
        { icon: 'eye-off',  label: 'hide', key: 'h', active: false,      color: T.red,   action: () => { hideItem(item!.id); onBack(); } },
      ] as btn}
        <button
          onclick={btn.action}
          style="flex:1;display:flex;flex-direction:column;align-items:center;gap:4px;padding:10px 0;background:transparent;border:none;color:{btn.active ? btn.color : T.ink2};cursor:pointer;font:9px/1 {T.mono};letter-spacing:0.4px;min-height:52px;"
          title={btn.icon === 'ext' && item.domain ? `Open https://${item.domain}` : undefined}
        >
          <div style="display:flex;align-items:center;gap:4px;">
            <Icon name={btn.icon} size={16} color={btn.active ? btn.color : T.ink1} />
            <KeyCap k={btn.key} dim />
          </div>
          <span style="text-transform:uppercase;">{btn.label}</span>
        </button>
      {/each}
    </div>

    <!-- Read time strip -->
    <div style="display:flex;align-items:center;justify-content:flex-end;padding:5px 10px;border-top:1px solid {T.bd0};background:{T.bg1};font:10px/1 {T.mono};color:{T.ink2};flex-shrink:0;">
      <span>~{Math.max(1, Math.round((item.body || '').split(/\s+/).filter(Boolean).length / 238))}min read</span>
    </div>

    <!-- Explain popover (bottom sheet) -->
    {#if popoverTag}
      {@const c = TAG_COLORS[popoverTag] ?? TAG_COLORS['low-effort']}
      {@const evidence = TAG_EVIDENCE[popoverTag] ?? ['title-token match', 'body-token match']}
      <div
        role="button"
        tabindex="-1"
        onclick={() => { popoverTag = null; }}
        onkeydown={(e) => { if (e.key === 'Escape') popoverTag = null; }}
        style="position:absolute;inset:0;background:rgba(0,0,0,0.55);display:flex;align-items:flex-end;z-index:20;"
      >
        <div
          role="button"
          tabindex="-1"
          onclick={(e) => e.stopPropagation()}
          onkeydown={() => {}}
          style="width:100%;background:{T.bg2};border-top:1px solid {c.bd};padding:14px 14px 24px;font:12px/1.4 {T.sans};color:{T.ink0};"
        >
          <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:10px;">
            <div style="display:flex;align-items:center;gap:8px;">
              <TagChip tag={popoverTag} size={11} />
              <span style="font:10px/1 {T.mono};color:{T.ink3};">tagged by {aiStatus.modelName ?? aiStatus.taggingMode} · {Math.round((item.aiScore ?? 0.8) * 100)}% conf</span>
            </div>
            <button onclick={() => { popoverTag = null; }} style="background:transparent;border:none;color:{T.ink2};cursor:pointer;display:flex;">
              <Icon name="x" size={14} />
            </button>
          </div>
          <div style="color:{T.ink1};margin-bottom:8px;">Why tagged <b style="color:{c.fg};">{popoverTag}</b>:</div>
          <ul style="margin:0;padding:0 0 0 14px;color:{T.ink1};font:12px/1.55 {T.sans};">
            {#each evidence as ev}
              <li style="margin-bottom:2px;">{ev}</li>
            {/each}
          </ul>
          <div style="margin-top:12px;padding-top:10px;border-top:1px solid {T.bd1};display:flex;gap:8px;">
            <button style="flex:1;padding:10px 0;background:transparent;color:{T.ink1};border:1px solid {T.bd2};border-radius:3px;font:11px/1 {T.mono};cursor:pointer;letter-spacing:0.3px;">flag wrong tag</button>
            <button style="flex:1;padding:10px 0;background:transparent;color:{T.ink1};border:1px solid {T.bd2};border-radius:3px;font:11px/1 {T.mono};cursor:pointer;letter-spacing:0.3px;">filter out "{popoverTag}"</button>
          </div>
        </div>
      </div>
    {/if}
  </div>
{:else}
  <div style="height:100%;display:flex;align-items:center;justify-content:center;color:{T.ink3};font:11px/1 {T.mono};">
    item not found
  </div>
{/if}
