<script lang="ts">
  import { T } from '$lib/tokens';
  import { sources, groups, coldstartTiming, dbStats, addSource, createGroup } from '$lib/stores/data.svelte';
  import { settings } from '$lib/settings.svelte';
  import { logger } from '$lib/logger';
  import Icon from '$lib/components/Icon.svelte';
  import KeyCap from '$lib/components/KeyCap.svelte';
  import ThemeSection from '$lib/components/ThemeSection.svelte';
  import SettingsSection from '$lib/components/SettingsSection.svelte';
  import ToggleSwitch from '$lib/components/ToggleSwitch.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import GhostButton from '$lib/components/shared/GhostButton.svelte';
  import { version } from '$app/environment';
  import { openExternal, shareItem } from '$lib/utils';

  let { showShortcuts = false }: { showShortcuts?: boolean } = $props();

  const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  // IS_DESKTOP: true when running in Tauri on a non-mobile platform.
  // navigator.maxTouchPoints > 1 is a reasonable heuristic used elsewhere in the app.
  const IS_DESKTOP = IS_TAURI && (typeof navigator === 'undefined' || navigator.maxTouchPoints <= 1);

  async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }

  const okCount     = $derived(sources.filter(s => s.status === 'ok').length);
  const errCount    = $derived(sources.filter(s => s.status === 'error').length);

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

<SettingsSection label="overview">
  <div class="grid grid-cols-2 gap-2">
    {#each [
      { label: 'items',       val: String(dbStats.totalItems),   color: T.cyan  },
      { label: 'unread',      val: String(dbStats.unreadItems),  color: T.cyan  },
      { label: 'saved',       val: String(dbStats.savedItems),   color: T.amber },
      { label: 'sources ok',  val: `${okCount}/${sources.length}`, color: errCount > 0 ? T.amber : T.green },
    ] as stat}
      <div class="p-2 bg-bg-0 border border-bd-0 rounded">
        <div class="tabular-nums text-[16px] leading-none font-mono" style="color:{stat.color};">{stat.val}</div>
        <div class="mt-1.5 text-ink-3 text-[10px] leading-none font-mono">{stat.label}</div>
      </div>
    {/each}
  </div>
</SettingsSection>

<SettingsSection label="appearance">
  <ThemeSection />
</SettingsSection>

<SettingsSection label="reading">
  <div class="flex flex-col gap-2.5">
    <div>
      <div class="text-ink-1 mb-1.5 text-[11px] leading-none font-mono">Mark as read</div>
      <SegmentedControl options={['open','never']} active={settings.markReadOn} onChange={v => { settings.markReadOn = v as typeof settings.markReadOn; }} />
    </div>
  </div>
</SettingsSection>

<SettingsSection label="sync">
  <div class="flex flex-col gap-2.5">
    <div>
      <div class="text-ink-1 mb-1.5 text-[11px] leading-none font-mono">Interval (minutes)</div>
      <SegmentedControl options={['5','15','30','60']} active={String(settings.syncIntervalMin)} onChange={v => { settings.syncIntervalMin = Number(v) as typeof settings.syncIntervalMin; }} />
    </div>
    <div class="flex items-center gap-2">
      <span class="flex-1 text-ink-1 text-[11px] leading-none font-mono">Wi-Fi only</span>
      <ToggleSwitch on={settings.wifiOnly} change={() => { settings.wifiOnly = !settings.wifiOnly; }} ariaLabel="Sync only on Wi-Fi" />
    </div>
    <div class="flex items-center gap-2">
      <span class="flex-1 text-ink-1 text-[11px] leading-none font-mono">Background sync</span>
      <ToggleSwitch on={settings.backgroundSync} change={() => { settings.backgroundSync = !settings.backgroundSync; }} ariaLabel="Background sync" />
    </div>
  </div>
</SettingsSection>

<!-- Tagging -->
<SettingsSection label="tagging">
  <div class="text-ink-3 text-[10px] leading-normal font-mono">Items are tagged on-device by a deterministic rule engine (structural rules + low-effort heuristics). No models, no downloads, no cloud.</div>
</SettingsSection>

<!-- Notifications -->
<SettingsSection label="notifications">
  <div class="flex flex-col gap-2.5">
    <div class="flex items-center gap-2">
      <span class="flex-1 text-ink-1 text-[11px] leading-none font-mono">High-signal items</span>
      <ToggleSwitch on={settings.notifyHighSignal} change={() => { settings.notifyHighSignal = !settings.notifyHighSignal; }} ariaLabel="Notify on high-signal items" />
    </div>
    <div class="flex items-center gap-2">
      <span class="flex-1 text-ink-1 text-[11px] leading-none font-mono">Saved item updates</span>
      <ToggleSwitch on={settings.notifySaved} change={() => { settings.notifySaved = !settings.notifySaved; }} ariaLabel="Notify when items are saved" />
    </div>
  </div>
</SettingsSection>

<!-- Keyboard shortcuts (desktop only) -->
{#if showShortcuts}
  <SettingsSection label="keyboard shortcuts">
    <div class="flex flex-col gap-1.5">
      {#each [['j / k', 'navigate items'], ['m', 'toggle read'], ['s', 'save / unsave'], ['o', 'open link'], ['x', 'hide item'], ['/', 'focus search'], ['Esc', 'clear / close']] as [k, label]}
        <div class="flex items-center gap-2 text-ink-2 text-[10px] leading-none font-mono">
          <KeyCap {k} dim />
          <span>{label}</span>
        </div>
      {/each}
    </div>
  </SettingsSection>
{/if}

<!-- Storage -->
<SettingsSection label="storage">
  <div class="text-ink-1 text-[11px] leading-[1.4] font-mono">{dbStats.totalItems} items · {sources.length} sources</div>
  <div class="mt-1 text-ink-3 text-[10px] leading-[1.4] font-mono">SQLite WAL{dbStats.dbSizeKb > 0 ? ` · ${dbStats.dbSizeKb >= 1024 ? (dbStats.dbSizeKb/1024).toFixed(1)+' MB' : dbStats.dbSizeKb+' KB'}` : ''}</div>
</SettingsSection>

<!-- Diagnostics -->
{#if IS_TAURI}
<SettingsSection label="diagnostics">
  <div class="flex flex-col gap-2.5">

    <!-- Verbose logging toggle -->
    <div class="flex items-center gap-2">
      <div class="flex-1">
        <div class="text-ink-1 text-[11px] leading-none font-mono">Verbose logging</div>
        <div class="mt-0.75 text-ink-3 text-[10px] leading-[1.4] font-mono">Logs per-item tagging, sync steps, and inference calls. Enable before reproducing a bug.</div>
      </div>
      <ToggleSwitch on={settings.verboseLogging} change={() => { settings.verboseLogging = !settings.verboseLogging; }} ariaLabel="Verbose logging" />
    </div>

    <!-- Desktop: show log path + open folder -->
    {#if IS_DESKTOP}
      {#if logPath}
        <div class="text-ink-3 wrap-break-word text-[10px] leading-[1.4] font-mono">Logs: {logPath}</div>
      {/if}
      <button
        onclick={handleOpenLogsFolder}
        class="flex items-center gap-1.5 w-full py-2 px-2.5 bg-transparent border border-bd-1 rounded text-ink-1 cursor-pointer text-left text-[10px] leading-none font-mono"
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
        class="flex items-center justify-center gap-1.5 w-full py-2 px-2.5 bg-transparent border border-bd-1 rounded text-left text-[10px] leading-none font-mono" style="color:{sharingLogs ? T.ink3 : shareStatus === 'error' ? T.amber : T.ink1};cursor:{sharingLogs ? 'default' : 'pointer'};"
      >
        {#if sharingLogs}
          <span class="syncing"><Icon name="sync" size={11} color={T.ink3} /></span>
        {/if}
        {sharingLogs ? 'sharing…' : shareStatus === 'done' ? 'shared' : shareStatus === 'error' ? 'no logs yet' : 'Share recent logs'}
      </button>
    {/if}

  </div>
</SettingsSection>
{/if}

<!-- Performance -->
{#if IS_TAURI}
<SettingsSection label="performance">
  {#if coldstartTiming.data}
    {@const d = coldstartTiming.data}
    <div class="grid grid-cols-2 gap-2">
      {#each [
        { label: 'cold start',  val: `${d.totalMs} ms`, color: d.totalMs < 300 ? T.green : d.totalMs < 700 ? T.amber : T.red },
        { label: 'ipc latency', val: `${d.ipcMs} ms`,   color: d.ipcMs   < 200 ? T.green : d.ipcMs   < 500 ? T.amber : T.red },
        { label: 'items',       val: String(d.itemCount),   color: T.cyan },
        { label: 'sources',     val: String(d.sourceCount), color: T.cyan },
      ] as stat}
        <div class="p-2 bg-bg-0 border border-bd-0 rounded">
          <div class="tabular-nums text-[16px] leading-none font-mono" style="color:{stat.color};">{stat.val}</div>
          <div class="mt-1.5 text-ink-3 text-[10px] leading-none font-mono">{stat.label}</div>
        </div>
      {/each}
    </div>
    {#if d.attempt > 0}
      <div class="mt-2 text-amber text-[10px] leading-[1.4] font-mono">loaded on retry {d.attempt} (bridge delay: {d.waitMs} ms)</div>
    {/if}
  {:else}
    <div class="text-ink-3 text-[10px] leading-[1.4] font-mono">loading…</div>
  {/if}
</SettingsSection>
{/if}

<!-- About -->
<SettingsSection label="about">
  <div class="text-ink-2 text-[11px] leading-normal font-mono">Pulse <span class="text-cyan">{version}</span></div>
  <div class="mt-0.5 text-ink-3 text-[10px] leading-normal font-mono">Tauri 2 · Svelte 5 · Rust · MIT</div>
  <div class="mt-2.5 flex flex-col gap-1.5">
    <GhostButton
      onclick={() => openExternal('https://github.com/80avin/pulse-rs')}
      class="flex items-center gap-1.5 p-0 text-cyan text-left text-[10px] leading-none"
    >
      <Icon name="ext" size={11} color={T.cyan} />
      github.com/80avin/pulse-rs
    </GhostButton>
    <GhostButton
      onclick={() => openExternal('https://github.com/80avin/pulse-rs/issues')}
      class="flex items-center gap-1.5 p-0 text-ink-2 text-left text-[10px] leading-none"
    >
      <Icon name="ext" size={11} color={T.ink2} />
      report an issue
    </GhostButton>
  </div>
  <div class="mt-2.5 text-ink-3 text-[10px] leading-[1.4] font-mono">No telemetry. All data stays on your device.</div>
</SettingsSection>

<!-- Advanced -->
<SettingsSection label="advanced">
  <GhostButton
    onclick={() => { showAdvanced = !showAdvanced; if (showAdvanced) exportSources(); }}
    class="flex items-center gap-1.5 p-0 text-left"
  >
    <Icon name={showAdvanced ? 'chev-dn' : 'chev-r'} size={10} color={T.ink3} />
    <span class="text-ink-3 text-[10px] leading-none font-mono">{showAdvanced ? 'hide' : 'show'} import / export</span>
  </GhostButton>

  {#if showAdvanced}
    <div class="mt-3 flex flex-col gap-2.5">
      <div class="text-ink-3 text-[10px] leading-[1.4] font-mono">Export your sources as JSON to back them up or share them. Paste JSON below to import.</div>

      <textarea
        bind:value={sourceJson}
        placeholder={importPlaceholder}
        rows={6}
        class="w-full p-2 bg-bg-0 border border-bd-1 rounded text-ink-1 resize-y outline-none box-border text-[10px] leading-[1.4] font-mono"
      ></textarea>

      <div class="flex gap-2">
        <button
          onclick={handleShareExport}
          class="flex-1 p-2 bg-transparent border border-bd-1 rounded text-ink-1 cursor-pointer flex items-center justify-center gap-1.5 text-[10px] leading-none font-mono"
        >
          <Icon name="share" size={11} color={T.ink2} />
          share
        </button>
        <button
          onclick={handleImport}
          disabled={importStatus === 'loading'}
          class="flex-1 p-2 border border-bd-1 rounded flex items-center justify-center gap-1.5 text-[10px] leading-none font-mono" style="background:{importStatus === 'loading' ? T.bg0 : 'transparent'};color:{importStatus === 'loading' ? T.ink3 : T.ink1};cursor:{importStatus === 'loading' ? 'default' : 'pointer'};"
        >
          <Icon name="import" size={11} color={importStatus === 'loading' ? T.ink3 : T.ink2} />
          {importStatus === 'loading' ? 'importing…' : 'import'}
        </button>
      </div>

      {#if importMsg}
        <div class="text-[10px] leading-none font-mono" style="color:{importStatus === 'error' ? T.red : T.green};">{importMsg}</div>
      {/if}
    </div>
  {/if}
</SettingsSection>
