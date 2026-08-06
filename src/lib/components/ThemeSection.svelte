<script lang="ts">
  import { T } from '$lib/tokens';
  import { settings } from '$lib/settings.svelte';
  import Icon from './Icon.svelte';

  const TAB_BTN = 'flex-1 flex items-center justify-center gap-2 rounded cursor-pointer focus-visible:ring-2 focus-visible:ring-[var(--color-cyan)] focus-visible:outline-none';
  const TOGGLE_BTN = 'flex-1 rounded cursor-pointer focus-visible:ring-2 focus-visible:ring-[var(--color-cyan)] focus-visible:outline-none';

  const ACCENT_OPTIONS = [
    { id: 'cyan',   light: '#155e75', dark: '#4ecdd6', label: 'Cyan' },
    { id: 'blue',   light: '#1d4ed8', dark: '#60a5fa', label: 'Blue' },
    { id: 'green',  light: '#166534', dark: '#6bd896', label: 'Green' },
    { id: 'amber',  light: '#92400e', dark: '#e6b450', label: 'Amber' },
    { id: 'violet', light: '#5b21b6', dark: '#b48ce6', label: 'Violet' },
  ];
</script>

<div>
  <div class="flex flex-col gap-2">
    <div class="flex items-center justify-between">
      <span class="text-[11px] leading-none font-mono text-ink-1">Theme</span>
    </div>
    <div class="flex gap-1.5" role="group" aria-label="Theme mode">
      <button
        onclick={() => { settings.theme = 'system'; }}
        aria-pressed={settings.theme === 'system'}
        class={TAB_BTN + ' p-[8px_6px] text-[10px] leading-none font-mono'}
        style="border:1px solid {settings.theme==='system' ? T.cyan : T.bd1};background:{settings.theme==='system' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='system' ? T.cyan : T.ink2};"
      >
        <Icon name="sync" size={12} color={settings.theme === 'system' ? T.cyan : T.ink2} />
        System
      </button>
      <button
        onclick={() => { settings.theme = 'dark'; }}
        aria-pressed={settings.theme === 'dark'}
        class={TAB_BTN + ' p-[8px_6px] text-[10px] leading-none font-mono'}
        style="border:1px solid {settings.theme==='dark' ? T.cyan : T.bd1};background:{settings.theme==='dark' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='dark' ? T.cyan : T.ink2};"
      >
        <Icon name="moon" size={12} color={settings.theme === 'dark' ? T.cyan : T.ink2} />
        Dark
      </button>
      <button
        onclick={() => { settings.theme = 'light'; }}
        aria-pressed={settings.theme === 'light'}
        class={TAB_BTN + ' p-[8px_6px] text-[10px] leading-none font-mono'}
        style="border:1px solid {settings.theme==='light' ? T.cyan : T.bd1};background:{settings.theme==='light' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='light' ? T.cyan : T.ink2};"
      >
        <Icon name="sun" size={12} color={settings.theme === 'light' ? T.cyan : T.ink2} />
        Light
      </button>
    </div>
  </div>

  <div class="flex flex-col gap-2 mt-4">
    <div class="flex items-center justify-between">
      <span class="text-[11px] leading-none font-mono text-ink-1">Accent Color</span>
    </div>
    <div class="flex gap-1.5" role="group" aria-label="Accent color">
      {#each ACCENT_OPTIONS as opt}
        <button
          onclick={() => { settings.accentColor = opt.id; }}
          aria-label={opt.label}
          aria-pressed={settings.accentColor === opt.id}
          class={TOGGLE_BTN}
          style="height:32px;border:2px solid {settings.accentColor===opt.id ? opt.dark : T.bd1};background:{`${opt.dark}cc`};"
        >
        </button>
      {/each}
    </div>
  </div>

  <div class="flex flex-col gap-2 mt-4">
    <div class="flex items-center justify-between">
      <span class="text-[11px] leading-none font-mono text-ink-1">Density</span>
    </div>
    <div class="flex gap-1.5" role="group" aria-label="Density">
      {#each (['dense', 'normal', 'roomy'] as const) as d}
        <button
          onclick={() => { settings.density = d; }}
          aria-pressed={settings.density === d}
          class={TOGGLE_BTN + ' p-[6px_8px] text-[10px] leading-none font-mono uppercase tracking-[0.3px]'}
          style="border:1px solid {settings.density===d ? T.cyan : T.bd1};background:{settings.density===d ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.density===d ? T.cyan : T.ink2};"
        >
          {d}
        </button>
      {/each}
    </div>
  </div>
</div>
