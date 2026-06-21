<script lang="ts">
  import { onMount } from 'svelte';
  import '../app.css';
  import { setupTaggingListener } from '$lib/stores/ai.svelte';
  import { initStore } from '$lib/stores/data.svelte';
  import { logger } from '$lib/logger';
  import { setupShareListener, shareSheet } from '$lib/share.svelte';
  import { initSettings, settings } from '$lib/settings.svelte';
  import ShareSheet from '$lib/components/ShareSheet.svelte';

  let { children } = $props();

  onMount(() => {
    Promise.all([initStore(), initSettings()]).catch(e => logger.error('app init failed', e));
  });

  $effect(() => {
    let unlisten: (() => void) | undefined;
    setupTaggingListener().then(u => { unlisten = u; });
    return () => { unlisten?.(); };
  });

  $effect(() => {
    let unlisten: (() => void) | undefined;
    setupShareListener().then(u => { unlisten = u; });
    return () => { unlisten?.(); };
  });

  // System theme detection via prefers-color-scheme
  let systemPrefersDark = $state(
    typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches
  );

  $effect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    function onChange(e: MediaQueryListEvent) { systemPrefersDark = e.matches; }
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  // Resolved actual theme: 'system' → OS preference, otherwise use explicit setting
  const resolvedTheme = $derived(
    settings.theme === 'system' ? (systemPrefersDark ? 'dark' : 'light') : settings.theme
  );

  const themeStyle = $derived(`height:100%;display:flex;flex-direction:column`);
  const densityAttr = $derived(settings.density);
  const accentColor = $derived(settings.accentColor);

  // Apply theme to <html> so portal content (Dialog.Portal) also inherits it
  $effect(() => {
    document.documentElement.style.colorScheme = resolvedTheme;
    document.documentElement.dataset.theme = resolvedTheme;
  });

  const ACCENTS: Record<string, { light: string; dark: string; lightDim: string; darkDim: string }> = {
    cyan:   { light: '#155e75', dark: '#4ecdd6', lightDim: '#0e7490', darkDim: '#2a7a82' },
    blue:   { light: '#1d4ed8', dark: '#60a5fa', lightDim: '#1e40af', darkDim: '#2563eb' },
    green:  { light: '#166534', dark: '#6bd896', lightDim: '#14532d', darkDim: '#2f6a44' },
    amber:  { light: '#92400e', dark: '#e6b450', lightDim: '#78350f', darkDim: '#7a5e2a' },
    violet: { light: '#5b21b6', dark: '#b48ce6', lightDim: '#4c1d95', darkDim: '#7b4fbf' },
  };

  const accentVars = $derived(() => {
    const a = ACCENTS[accentColor] ?? ACCENTS.cyan;
    return `:root{--user-accent:light-dark(${a.light},${a.dark});--user-accent-dim:light-dark(${a.lightDim},${a.darkDim})}`;
  });
</script>

<svelte:head>
  {@html `<style>${accentVars()}</style>`}
</svelte:head>

<a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-3 focus:py-2 focus:bg-bg-0 focus:text-ink-0 focus:border focus:border-cyan focus:rounded">
  Skip to content
</a>

<div style={themeStyle} data-theme={resolvedTheme} data-density={densityAttr} id="main-content">
  {@render children()}
  {#if shareSheet.candidate !== null}
    <ShareSheet />
  {/if}
</div>
