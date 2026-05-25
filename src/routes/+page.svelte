<script lang="ts">
  import { onMount } from 'svelte';
  import Desktop from '$lib/screens/Desktop.svelte';
  import Mobile from '$lib/screens/Mobile.svelte';

  // null until onMount determines the viewport — prevents Desktop from
  // mounting and tearing down immediately on Android.
  let isMobile = $state<boolean | null>(null);

  onMount(() => {
    const check = () => { isMobile = window.innerWidth < 768; };
    check();
    window.addEventListener('resize', check);
    return () => window.removeEventListener('resize', check);
  });
</script>

{#if isMobile === true}
  <Mobile />
{:else if isMobile === false}
  <Desktop />
{/if}
