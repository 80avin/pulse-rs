<script lang="ts">
  import { onMount } from 'svelte';
  import Desktop from '$lib/screens/Desktop.svelte';
  import Mobile from '$lib/screens/Mobile.svelte';

  // Mobile: width < 768px (Android WebView will report real device width)
  let isMobile = $state(false);

  onMount(() => {
    const check = () => { isMobile = window.innerWidth < 768; };
    check();
    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
  });
</script>

{#if isMobile}
  <Mobile />
{:else}
  <Desktop />
{/if}
