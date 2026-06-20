<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources, groups, clearItems, loadMockData, aiStatus, coldstartTiming, dbStats, addSource, createGroup } from '$lib/store.svelte';
  import { settings } from '$lib/settings.svelte';
  import { logger } from '$lib/logger';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import { version } from '$app/environment';
  import { openExternal, shareItem } from '$lib/utils';

  let { showShortcuts = false }: { showShortcuts?: boolean } = $props();

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI__' in window;
  // IS_DESKTOP: true when running in Tauri on a non-mobile platform.
  // navigator.maxTouchPoints > 1 is a reasonable heuristic used elsewhere in the app.
  const IS_DESKTOP = IS_TAURI && (typeof navigator === 'undefined' || navigator.maxTouchPoints <= 1);

  async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }

  const okCount     = $derived(sources.filter(s => s.status === 'ok').length);
  const errCount    = $derived(sources.filter(s => s.status === 'error').length);

  let clearing = $state(false);

  // Diagnostics state
  let logPath = $state('');
  let sharingLogs = $state(false);
  let shareStatus = $state<'idle' | 'done' | 'error'>('idle');

  $effect(() => {
    if (!IS_TAURI) return;
    if (IS_DESKTOP) {
      tauriInvoke<string>('get_log_path').then(p => { logPath = p; }).catch(() => {});
    }
  });

  async function handleClearItems() {
    if (!confirm('Delete all cached items? Sources will remain. Re-sync to restore.')) return;
    clearing = true;
    await clearItems();
    clearing = false;
  }

  async function handleOpenLogsFolder() {
    await tauriInvoke('open_logs_folder').catch(e => logger.warn('open_logs_folder failed', e));
  }

  async function handleShareLogs() {
    sharingLogs = true;
    shareStatus = 'idle';
    try {
      await tauriInvoke('share_log_file');
      shareStatus = 'done';
      setTimeout(() => { shareStatus = 'idle'; }, 2500);
    } catch (e: any) {
      logger.warn('share logs failed', e);
      shareStatus = 'error';
      setTimeout(() => { shareStatus = 'idle'; }, 2500);
    } finally {
      sharingLogs = false;
    }
  }

  // ── Advanced: import/export sources ──────────────────────────────────
  let showAdvanced = $state(false);
  let sourceJson = $state('');
  let importStatus = $state<'idle' | 'loading' | 'done' | 'error'>('idle');
  let importMsg = $state('');
  const importPlaceholder = '[{"name":"Hacker News","url":"https://...","kind":"hn","group":"all"}]';

  function exportSources() {
    const data = sources.map(s => ({
      name: s.name,
      url: s.url,
      kind: s.kind,
      group: s.group,
    }));
    sourceJson = JSON.stringify(data, null, 2);
  }

  async function handleCopyExport() {
    if (!sourceJson) exportSources();
    try {
      await navigator.clipboard.writeText(sourceJson);
    } catch (e) {
      logger.warn('clipboard write failed', e);
    }
  }

  async function handleShareExport() {
    if (!sourceJson) exportSources();
    await shareItem('Pulse sources', undefined, sourceJson);
  }

  async function handleImport() {
    if (!sourceJson.trim()) return;
    importStatus = 'loading';
    importMsg = '';
    try {
      const parsed = JSON.parse(sourceJson);
      if (!Array.isArray(parsed)) throw new Error('Expected a JSON array of sources');
      const existingGroupIds = new Set(groups.map(g => g.id));
      const neededGroups = new Set<string>();
      for (const entry of parsed) {
        const g = (entry.group || '').trim();
        if (g && g !== 'all' && !existingGroupIds.has(g)) neededGroups.add(g);
      }
      for (const g of neededGroups) await createGroup(g);
      const before = sources.length;
      for (const entry of parsed) {
        if (!entry.url || !entry.name) continue;
        const kind = (['rss', 'hn', 'reddit'].includes(entry.kind) ? entry.kind : 'rss') as 'rss' | 'hn' | 'reddit';
        const group = entry.group || 'all';
        try {
          await addSource(entry.name, entry.url, kind, group);
        } catch (e) {
          logger.warn(`import: failed to add "${entry.name}"`, e);
        }
      }
      const realAdded = sources.length - before;
      importMsg = `Imported ${realAdded} source${realAdded !== 1 ? 's' : ''}.`;
      importStatus = 'done';
      setTimeout(() => { importStatus = 'idle'; importMsg = ''; }, 4000);
    } catch (e: any) {
      importMsg = e?.message ?? 'Invalid JSON';
      importStatus = 'error';
      setTimeout(() => { importStatus = 'idle'; importMsg = ''; }, 4000);
    }
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
      { label: 'items',       val: String(dbStats.totalItems),   color: T.cyan  },
      { label: 'unread',      val: String(dbStats.unreadItems),  color: T.cyan  },
      { label: 'saved',       val: String(dbStats.savedItems),   color: T.amber },
      { label: 'sources ok',  val: `${okCount}/${sources.length}`, color: errCount > 0 ? T.amber : T.green },
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
  <div style="font:11px/1.4 {T.mono};color:{T.ink1};">{dbStats.totalItems} items · {sources.length} sources</div>
  <div style="margin-top:4px;font:10px/1.4 {T.mono};color:{T.ink3};">SQLite WAL{dbStats.dbSizeKb > 0 ? ` · ${dbStats.dbSizeKb >= 1024 ? (dbStats.dbSizeKb/1024).toFixed(1)+' MB' : dbStats.dbSizeKb+' KB'}` : ''}</div>
</div>

<!-- Diagnostics -->
{#if IS_TAURI}
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">diagnostics</div>
  <div style="display:flex;flex-direction:column;gap:10px;">

    <!-- Verbose logging toggle -->
    <div style="display:flex;align-items:center;gap:8px;">
      <div style="flex:1;">
        <div style="font:11px/1 {T.mono};color:{T.ink1};">Verbose logging</div>
        <div style="margin-top:3px;font:9px/1.4 {T.mono};color:{T.ink3};">Logs per-item tagging, sync steps, and inference calls. Enable before reproducing a bug.</div>
      </div>
      {@render toggle(settings.verboseLogging, () => { settings.verboseLogging = !settings.verboseLogging; })}
    </div>

    <!-- Desktop: show log path + open folder -->
    {#if IS_DESKTOP}
      {#if logPath}
        <div style="font:9px/1.4 {T.mono};color:{T.ink3};word-break:break-all;">Logs: {logPath}</div>
      {/if}
      <button
        onclick={handleOpenLogsFolder}
        style="display:flex;align-items:center;gap:6px;width:100%;padding:8px 10px;background:transparent;border:1px solid {T.bd1};border-radius:3px;font:10px/1 {T.mono};color:{T.ink1};cursor:pointer;text-align:left;"
      >
        <Icon name="ext" size={11} color={T.ink2} />
        Open logs folder
      </button>
    {/if}

    <!-- Mobile: share logs -->
    {#if !IS_DESKTOP}
      <button
        onclick={handleShareLogs}
        disabled={sharingLogs}
        style="display:flex;align-items:center;justify-content:center;gap:6px;width:100%;padding:8px 10px;background:transparent;border:1px solid {T.bd1};border-radius:3px;font:10px/1 {T.mono};color:{sharingLogs ? T.ink3 : shareStatus === 'error' ? T.amber : T.ink1};cursor:{sharingLogs ? 'default' : 'pointer'};"
      >
        {#if sharingLogs}
          <span class="syncing"><Icon name="sync" size={11} color={T.ink3} /></span>
        {/if}
        {sharingLogs ? 'sharing…' : shareStatus === 'done' ? 'shared' : shareStatus === 'error' ? 'no logs yet' : 'Share recent logs'}
      </button>
    {/if}

  </div>
</div>
{/if}

<!-- Performance -->
{#if IS_TAURI}
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;margin-bottom:10px;">performance</div>
  {#if coldstartTiming.data}
    {@const d = coldstartTiming.data}
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;">
      {#each [
        { label: 'cold start',  val: `${d.totalMs} ms`, color: d.totalMs < 300 ? T.green : d.totalMs < 700 ? T.amber : T.red },
        { label: 'ipc latency', val: `${d.ipcMs} ms`,   color: d.ipcMs   < 200 ? T.green : d.ipcMs   < 500 ? T.amber : T.red },
        { label: 'items',       val: String(d.itemCount),   color: T.cyan },
        { label: 'sources',     val: String(d.sourceCount), color: T.cyan },
      ] as stat}
        <div style="padding:8px;background:{T.bg0};border:1px solid {T.bd0};border-radius:3px;">
          <div style="font:16px/1 {T.mono};color:{stat.color};font-variant-numeric:tabular-nums;">{stat.val}</div>
          <div style="margin-top:5px;font:9px/1 {T.mono};color:{T.ink3};">{stat.label}</div>
        </div>
      {/each}
    </div>
    {#if d.attempt > 0}
      <div style="margin-top:8px;font:9px/1.4 {T.mono};color:{T.amber};">loaded on retry {d.attempt} (bridge delay: {d.waitMs} ms)</div>
    {/if}
  {:else}
    <div style="font:10px/1.4 {T.mono};color:{T.ink3};">loading…</div>
  {/if}
</div>
{/if}

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

<!-- Advanced -->
<div style="padding:12px;background:{T.bg1};border:1px solid {T.bd0};border-radius:4px;">
  <button
    onclick={() => { showAdvanced = !showAdvanced; if (showAdvanced) exportSources(); }}
    style="display:flex;align-items:center;gap:6px;width:100%;background:transparent;border:none;cursor:pointer;padding:0;text-align:left;"
  >
    <Icon name={showAdvanced ? 'chev-dn' : 'chev-r'} size={10} color={T.ink3} />
    <div style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.6px;text-transform:uppercase;">advanced</div>
  </button>

  {#if showAdvanced}
    <div style="margin-top:12px;display:flex;flex-direction:column;gap:10px;">
      <div style="font:9px/1.4 {T.mono};color:{T.ink3};">Export your sources as JSON to back them up or share them. Paste JSON below to import.</div>

      <textarea
        bind:value={sourceJson}
        placeholder={importPlaceholder}
        rows={6}
        style="width:100%;padding:8px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:10px/1.4 {T.mono};color:{T.ink1};resize:vertical;outline:none;box-sizing:border-box;"
      ></textarea>

      <div style="display:flex;gap:8px;">
        <button
          onclick={handleShareExport}
          style="flex:1;padding:8px;background:transparent;border:1px solid {T.bd1};border-radius:3px;font:10px/1 {T.mono};color:{T.ink1};cursor:pointer;display:flex;align-items:center;justify-content:center;gap:6px;"
        >
          <Icon name="share" size={11} color={T.ink2} />
          share
        </button>
        <button
          onclick={handleImport}
          disabled={importStatus === 'loading'}
          style="flex:1;padding:8px;background:{importStatus === 'loading' ? T.bg0 : 'transparent'};border:1px solid {T.bd1};border-radius:3px;font:10px/1 {T.mono};color:{importStatus === 'loading' ? T.ink3 : T.ink1};cursor:{importStatus === 'loading' ? 'default' : 'pointer'};display:flex;align-items:center;justify-content:center;gap:6px;"
        >
          <Icon name="import" size={11} color={importStatus === 'loading' ? T.ink3 : T.ink2} />
          {importStatus === 'loading' ? 'importing…' : 'import'}
        </button>
      </div>

      {#if importMsg}
        <div style="font:9px/1 {T.mono};color:{importStatus === 'error' ? T.red : T.green};">{importMsg}</div>
      {/if}
    </div>
  {/if}
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
