<script lang="ts">
  import { T } from '$lib/tokens';
  import { items, sources, clearItems, loadMockData, aiStatus } from '$lib/store.svelte';
  import { settings } from '$lib/settings.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import { version } from '$app/environment';
  import { openExternal } from '$lib/utils';

  let { showShortcuts = false }: { showShortcuts?: boolean } = $props();

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;
  async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }

  const okCount     = $derived(sources.filter(s => s.status === 'ok').length);
  const errCount    = $derived(sources.filter(s => s.status === 'error').length);
  const unreadCount = $derived(items.filter(i => !i.read).length);
  const savedCount  = $derived(items.filter(i => i.saved).length);

  let dbSizeKb = $state(0);
  let clearing = $state(false);

  $effect(() => {
    if (!IS_TAURI) return;
    tauriInvoke<{ dbSizeKb: number }>('get_db_stats')
      .then(s => { dbSizeKb = s.dbSizeKb; })
      .catch(() => {});
  });

  async function handleClearItems() {
    if (!confirm('Delete all cached items? Sources will remain. Re-sync to restore.')) return;
    clearing = true;
    await clearItems();
    dbSizeKb = 0;
    clearing = false;
  }
</script>

{#snippet toggle(on: boolean, change: () => void)}
  <button
    onclick={change}
    role="switch"
    aria-checked={on}
    aria-label={on ? 'on' : 'off'}
    style="width:44px;height:24px;border-radius:12px;border:none;cursor:pointer;background:{on ? T.cyan : T.bg3};position:relative;flex-shrink:0;transition:background 0.15s;"
  >
    <span style="position:absolute;top:3px;left:{on ? '23px' : '3px'};width:18px;height:18px;border-radius:9px;background:{on ? T.bg0 : T.ink3};transition:left 0.15s;"></span>
  </button>
{/snippet}

{#snippet seg(options: string[], active: string, change: (v: string) => void)}
  <div style="display:flex;gap:3px;background:{T.bg0};border:1px solid {T.bd1};border-radius:4px;padding:2px;">
    {#each options as opt}
      <button
        onclick={() => change(opt)}
        style="flex:1;padding:5px 4px;border:none;border-radius:3px;cursor:pointer;font:9px/1 {T.mono};letter-spacing:0.4px;text-transform:uppercase;background:{opt === active ? T.bg3 : 'transparent'};color:{opt === active ? T.cyan : T.ink2};"
      >{opt}</button>
    {/each}
  </div>
{/snippet}

<!-- Stats overview -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">overview</div>
  <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
    {#each [
      { label: 'cached items', val: String(items.length),           color: T.cyan  },
      { label: 'unread',       val: String(unreadCount),            color: T.cyan  },
      { label: 'saved',        val: String(savedCount),             color: T.amber },
      { label: 'sources ok',   val: `${okCount}/${sources.length}`, color: errCount > 0 ? T.amber : T.green },
    ] as stat}
      <div style="padding:8px;background:{T.bg0};border:1px solid {T.bd0};border-radius:3px;">
        <div style="font:16px/1 {T.mono};color:{stat.color};font-variant-numeric:tabular-nums;">{stat.val}</div>
        <div style="margin-top:5px;font:9px/1 {T.mono};color:{T.ink3};">{stat.label}</div>
      </div>
    {/each}
  </div>
</div>

<!-- Reading -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">reading</div>
  <div style="display:flex;flex-direction:column;gap:10px;">
    <div>
      <div style="font:11px/1 {T.mono};color:{T.ink1};margin-bottom:6px;">Density</div>
      {@render seg(['dense','normal','roomy'], settings.density, v => { settings.density = v as typeof settings.density; })}
    </div>
    <div>
      <div style="font:11px/1 {T.mono};color:{T.ink1};margin-bottom:6px;">Mark as read</div>
      {@render seg(['open','never'], settings.markReadOn, v => { settings.markReadOn = v as typeof settings.markReadOn; })}
    </div>
  </div>
</div>

<!-- Sync -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">sync</div>
  <div style="display:flex;flex-direction:column;gap:10px;">
    <div>
      <div style="font:11px/1 {T.mono};color:{T.ink1};margin-bottom:6px;">Interval (minutes)</div>
      {@render seg(['5','15','30','60'], String(settings.syncIntervalMin), v => { settings.syncIntervalMin = Number(v) as typeof settings.syncIntervalMin; })}
    </div>
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">Wi-Fi only</span>
      {@render toggle(settings.wifiOnly, () => { settings.wifiOnly = !settings.wifiOnly; })}
    </div>
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">Background sync</span>
      {@render toggle(settings.backgroundSync, () => { settings.backgroundSync = !settings.backgroundSync; })}
    </div>
  </div>
</div>

<!-- AI -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="display:flex;align-items:center;gap:8px;margin-bottom:10px;">
    <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;">ai tagging</div>
    <div style="font:8px/1 {T.mono};color:{T.amber};letter-spacing:0.5px;text-transform:uppercase;padding:2px 5px;border:1px solid {T.amber};border-radius:2px;opacity:0.8;">experimental</div>
  </div>
  <div style="margin-bottom:10px;font:9px/1.5 {T.mono};color:{T.ink3};">Tags may be inaccurate. Raise the confidence threshold or disable tagging if results look wrong.</div>
  <div style="display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">AI tagging</span>
      {@render toggle(settings.aiTagging, () => { settings.aiTagging = !settings.aiTagging; })}
    </div>
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">Model</span>
      <span style="font:11px/1 {T.mono};color:{aiStatus.taggingMode !== 'none' ? T.cyan : T.amber};">{aiStatus.taggingMode === 'none' ? 'not loaded' : aiStatus.taggingMode}</span>
    </div>
    <div>
      <div style="font:11px/1 {T.mono};color:{T.ink1};margin-bottom:6px;">Confidence threshold: <span style="color:{T.cyan};">{settings.confidenceThreshold.toFixed(2)}</span></div>
      <input type="range" min="0.1" max="0.9" step="0.05" bind:value={settings.confidenceThreshold} style="width:100%;accent-color:{T.cyan};" />
    </div>
  </div>
</div>

<!-- Notifications -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">notifications</div>
  <div style="display:flex;flex-direction:column;gap:10px;">
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">High-signal items</span>
      {@render toggle(settings.notifyHighSignal, () => { settings.notifyHighSignal = !settings.notifyHighSignal; })}
    </div>
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:11px/1 {T.mono};color:{T.ink1};">Saved item updates</span>
      {@render toggle(settings.notifySaved, () => { settings.notifySaved = !settings.notifySaved; })}
    </div>
  </div>
</div>

<!-- Keyboard shortcuts (desktop only) -->
{#if showShortcuts}
  <div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
    <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">keyboard shortcuts</div>
    <div style="display:flex;flex-direction:column;gap:6px;">
      {#each [['j / k', 'navigate items'], ['m', 'toggle read'], ['s', 'save / unsave'], ['o', 'open link'], ['h', 'hide item'], ['a', 'toggle ai panel'], ['/', 'focus search'], ['Esc', 'clear / close']] as [k, label]}
        <div style="display:flex;align-items:center;gap:8px;font:10px/1 {T.mono};color:{T.ink2};">
          <KeyCap {k} dim />
          <span>{label}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<!-- Storage + About -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">storage</div>
  <div style="font:11px/1.4 {T.mono};color:{T.ink1};">{items.length} items · {sources.length} sources</div>
  <div style="margin-top:4px;font:10px/1.4 {T.mono};color:{T.ink3};">SQLite WAL{dbSizeKb > 0 ? ` · ${dbSizeKb >= 1024 ? (dbSizeKb/1024).toFixed(1)+' MB' : dbSizeKb+' KB'}` : ''}</div>
</div>

<!-- About -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">about</div>
  <div style="font:11px/1.5 {T.mono};color:{T.ink2};">Pulse <span style="color:{T.cyan};">{version}</span></div>
  <div style="margin-top:2px;font:10px/1.5 {T.mono};color:{T.ink3};">Tauri 2 · Svelte 5 · Rust · MIT</div>
  <div style="margin-top:10px;display:flex;flex-direction:column;gap:6px;">
    <button
      onclick={() => openExternal('https://github.com/80avin/pulse-rs')}
      style="display:flex;align-items:center;gap:6px;background:transparent;border:none;cursor:pointer;padding:0;font:10px/1 {T.mono};color:{T.cyan};text-align:left;"
    >
      <Icon name="ext" size={11} color={T.cyan} />
      github.com/80avin/pulse-rs
    </button>
    <button
      onclick={() => openExternal('https://github.com/80avin/pulse-rs/issues')}
      style="display:flex;align-items:center;gap:6px;background:transparent;border:none;cursor:pointer;padding:0;font:10px/1 {T.mono};color:{T.ink2};text-align:left;"
    >
      <Icon name="ext" size={11} color={T.ink2} />
      report an issue
    </button>
  </div>
  <div style="margin-top:10px;font:9px/1.4 {T.mono};color:{T.ink3};">No telemetry. All data stays on your device.</div>
</div>

<!-- Actions -->
<div style="display:flex;flex-direction:column;gap:8px;">
  <button
    onclick={() => loadMockData()}
    style="display:flex;align-items:center;justify-content:center;gap:8px;width:100%;padding:12px;background:transparent;border:1px solid {T.bd1};border-radius:4px;font:12px/1 {T.mono};color:{T.amber};cursor:pointer;"
  >
    <Icon name="list" size={14} color={T.amber} />
    load sample data
  </button>
  <button
    onclick={handleClearItems}
    disabled={clearing}
    style="display:flex;align-items:center;justify-content:center;gap:8px;width:100%;padding:12px;background:transparent;border:1px solid {T.bd1};border-radius:4px;font:12px/1 {T.mono};color:{clearing ? T.ink3 : T.red};cursor:{clearing ? 'default' : 'pointer'};"
  >
    <Icon name="trash" size={14} color={clearing ? T.ink3 : T.red} />
    {clearing ? 'clearing…' : 'clear all cached items'}
  </button>
</div>
