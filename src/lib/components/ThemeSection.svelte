<script lang="ts">
  import { T, SOURCE_KIND } from '$lib/tokens';
  import { settings } from '$lib/settings.svelte';
  import Icon from './Icon.svelte';

  const ACCENT_OPTIONS = [
    { id: 'cyan',   light: '#155e75', dark: '#4ecdd6', label: 'Cyan' },
    { id: 'blue',   light: '#1d4ed8', dark: '#60a5fa', label: 'Blue' },
    { id: 'green',  light: '#166534', dark: '#6bd896', label: 'Green' },
    { id: 'amber',  light: '#92400e', dark: '#e6b450', label: 'Amber' },
    { id: 'violet', light: '#5b21b6', dark: '#b48ce6', label: 'Violet' },
  ];
</script>

<div>
  <!-- Theme mode -->
  <div style="display:flex;flex-direction:column;gap:8px;">
    <div style="display:flex;align-items:center;justify-content:space-between;">
      <label style="font:11px/1 {T.mono};color:{T.ink1};">Theme</label>
    </div>
    <div style="display:flex;gap:6px;" role="group" aria-label="Theme mode">
      <button
        onclick={() => { settings.theme = 'system'; }}
        aria-pressed={settings.theme === 'system'}
        class="flex-1 flex items-center justify-center gap-2 rounded cursor-pointer"
        style="padding:8px 6px;border:1px solid {settings.theme==='system' ? T.cyan : T.bd1};background:{settings.theme==='system' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='system' ? T.cyan : T.ink2};font:10px/1 {T.mono};"
      >
        <Icon name="sync" size={12} color={settings.theme === 'system' ? T.cyan : T.ink2} />
        System
      </button>
      <button
        onclick={() => { settings.theme = 'dark'; }}
        aria-pressed={settings.theme === 'dark'}
        class="flex-1 flex items-center justify-center gap-2 rounded cursor-pointer"
        style="padding:8px 6px;border:1px solid {settings.theme==='dark' ? T.cyan : T.bd1};background:{settings.theme==='dark' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='dark' ? T.cyan : T.ink2};font:10px/1 {T.mono};"
      >
        <Icon name="moon" size={12} color={settings.theme === 'dark' ? T.cyan : T.ink2} />
        Dark
      </button>
      <button
        onclick={() => { settings.theme = 'light'; }}
        aria-pressed={settings.theme === 'light'}
        class="flex-1 flex items-center justify-center gap-2 rounded cursor-pointer"
        style="padding:8px 6px;border:1px solid {settings.theme==='light' ? T.cyan : T.bd1};background:{settings.theme==='light' ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.theme==='light' ? T.cyan : T.ink2};font:10px/1 {T.mono};"
      >
        <Icon name="sun" size={12} color={settings.theme === 'light' ? T.cyan : T.ink2} />
        Light
      </button>
    </div>
  </div>

  <!-- Accent color -->
  <div style="display:flex;flex-direction:column;gap:8px;margin-top:16px;">
    <div style="display:flex;align-items:center;justify-content:space-between;">
      <label style="font:11px/1 {T.mono};color:{T.ink1};">Accent Color</label>
    </div>
    <div style="display:flex;gap:6px;" role="group" aria-label="Accent color">
      {#each ACCENT_OPTIONS as opt}
        <button
          onclick={() => { settings.accentColor = opt.id; }}
          aria-label={opt.label}
          aria-pressed={settings.accentColor === opt.id}
          class="flex-1 rounded cursor-pointer"
          style="height:32px;border:2px solid {settings.accentColor===opt.id ? opt.dark : T.bd1};background:{`${opt.dark}cc`};"
        >
        </button>
      {/each}
    </div>
  </div>

  <!-- Density -->
  <div style="display:flex;flex-direction:column;gap:8px;margin-top:16px;">
    <div style="display:flex;align-items:center;justify-content:space-between;">
      <label style="font:11px/1 {T.mono};color:{T.ink1};">Density</label>
    </div>
    <div style="display:flex;gap:6px;" role="group" aria-label="Density">
      {#each (['dense', 'normal', 'roomy'] as const) as d}
        <button
          onclick={() => { settings.density = d; }}
          aria-pressed={settings.density === d}
          class="flex-1 rounded cursor-pointer"
          style="padding:6px 8px;border:1px solid {settings.density===d ? T.cyan : T.bd1};background:{settings.density===d ? 'rgba(78,205,214,0.10)' : 'transparent'};color:{settings.density===d ? T.cyan : T.ink2};font:10px/1 {T.mono};text-transform:uppercase;letter-spacing:0.3px;"
        >
          {d}
        </button>
      {/each}
    </div>
  </div>
</div>
