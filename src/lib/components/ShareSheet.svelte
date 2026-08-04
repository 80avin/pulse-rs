<script lang="ts">
  import { T } from '$lib/tokens';
  import { Dialog } from 'bits-ui';
  import { groups } from '$lib/stores/data.svelte';
  import { shareSheet, dismissShare, confirmShare } from '$lib/share.svelte';
  import { openOverlay, closeOverlay } from '$lib/stores/overlays.svelte';
  import { isDesktop } from '$lib/use-is-desktop.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';

  let submitting = $state(false);

  $effect(() => {
    if (shareSheet.candidate !== null) {
      openOverlay('share-sheet');
      return () => closeOverlay('share-sheet');
    }
  });

  const noFeedFound = $derived(
    !shareSheet.loading &&
    shareSheet.candidate !== null &&
    shareSheet.candidate.noFeedFound &&
    !shareSheet.error
  );

  const creatingGroup = $derived(shareSheet.group === '__new__');

  async function handleConfirm() {
    if (submitting || !shareSheet.feedUrl) return;
    if (creatingGroup && !shareSheet.newGroupName.trim()) return;
    submitting = true;
    try {
      await confirmShare();
    } finally {
      submitting = false;
    }
  }
</script>

<Dialog.Root open={shareSheet.candidate !== null} onOpenChange={(open) => { if (!open) dismissShare(); }}>
  <Dialog.Portal>
    <Dialog.Overlay class="fixed inset-0 bg-black/60 z-[300]" />
    <Dialog.Content
      preventScroll={false}
      class="bg-bg-2 flex flex-col overflow-y-auto" style="position:fixed;{isDesktop() ? 'left:50%;top:50%;transform:translate(-50%,-50%);width:400px;max-width:90vw;border-radius:8px;' : 'bottom:0;left:0;right:0;width:100%;border-radius:16px 16px 0 0;'}padding:20px 16px max(20px,env(safe-area-inset-bottom));gap:14px;max-height:90vh;z-index:300;"
    >
      <!-- Header -->
      <div class="flex items-center gap-2">
        <Dialog.Title class="flex-1 font-semibold text-ink-0 m-0 tracking-[0.3px] text-[12px] leading-none font-mono">ADD FEED FROM SHARE</Dialog.Title>
        <Dialog.Close class="bg-transparent border-none cursor-pointer p-1 text-[16px] text-ink-2">&#x2715;</Dialog.Close>
      </div>

    {#if shareSheet.loading}
      <div class="text-center p-5 text-[11px] leading-none font-mono text-ink-3">detecting feed…</div>
    {:else}
      <!-- HN notice -->
      {#if shareSheet.candidate?.isHn}
        <div class="bg-bg-1 text-amber p-2 px-2.5 border-l-[3px] border-l-amber text-[10px] leading-normal font-mono">
          HN has one global feed — this subscribes to the front page.
        </div>
      {/if}

      <!-- Network / detection error -->
      {#if shareSheet.error}
        <div class="text-red text-[10px] leading-[1.4] font-mono">Could not reach this URL. Check connectivity or paste the feed URL directly.</div>
      {/if}

      <!-- Feed name -->
      <div class="flex flex-col gap-1.25">
        <label for="ss-name" class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">NAME</label>
        <input id="ss-name" bind:value={shareSheet.name} placeholder="Feed name"
          class="bg-bg-0 text-ink-0 w-full box-border p-2.5 rounded border border-bd-1 text-[12px] leading-none font-mono" />
      </div>

      <!-- Feed URL + no-feed warning -->
      <div class="flex flex-col gap-1.25">
        <label for="ss-url" class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">FEED URL</label>
        <input id="ss-url" bind:value={shareSheet.feedUrl} placeholder="https://…"
          class="bg-bg-0 text-ink-0 w-full box-border p-2.5 rounded text-[11px] leading-none font-mono" style="border:1px solid {noFeedFound ? T.red : T.bd1};" />
        {#if noFeedFound}
          <span class="text-red text-[9px] leading-[1.4] font-mono">No feed source found at this URL. Paste a direct RSS/Atom URL above if you have one.</span>
        {/if}
      </div>

      <!-- Alternate feed picker -->
      {#if (shareSheet.candidate?.candidates?.length ?? 0) > 1}
        <div class="flex flex-col gap-1">
          <span class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">ALTERNATE FEEDS</span>
          {#each shareSheet.candidate!.candidates as c}
            <button onclick={() => { shareSheet.feedUrl = c.url; if (c.title) shareSheet.name = c.title; }}
              class="text-left cursor-pointer p-2 rounded text-[10px] leading-[1.4] font-mono text-ink-1" style="background:{shareSheet.feedUrl===c.url?T.bg3:T.bg0};border:1px solid {shareSheet.feedUrl===c.url?T.cyan:T.bd1};">
              {c.title ?? c.url}
              <span class="block text-ink-3 truncate">{c.url}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Type + Group row -->
      <div class="flex gap-2">
        <!-- Type toggle -->
        <div class="flex-1 flex flex-col gap-1.25">
          <span class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">TYPE</span>
          <SegmentedControl options={['rss','hn','reddit']} active={shareSheet.kind} onChange={v => { shareSheet.kind = v as typeof shareSheet.kind; }} />
        </div>

        <!-- Group selector -->
        <div class="flex-1 flex flex-col gap-1.25">
          <label for="ss-group" class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">GROUP</label>
          <select id="ss-group" bind:value={shareSheet.group}
            class="w-full bg-bg-0 border border-bd-1 text-ink-0 p-2 rounded text-[11px] leading-none font-mono">
            {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
            <option value="__new__">+ new group</option>
          </select>
        </div>
      </div>

      <!-- New group name input (shown when __new__ selected) -->
      {#if creatingGroup}
        <div class="flex flex-col gap-1.25">
          <label for="ss-newgroup" class="text-ink-3 tracking-[0.5px] text-[9px] leading-none font-mono">NEW GROUP NAME</label>
          <input id="ss-newgroup"
            bind:value={shareSheet.newGroupName}
            placeholder="e.g. Tech, Local, Work…"
            class="bg-bg-0 text-ink-0 w-full box-border p-2.5 rounded border border-cyan text-[12px] leading-none font-mono"
          />
        </div>
      {/if}

      <!-- Action buttons -->
      <div class="flex gap-2 mt-1">
        <button onclick={dismissShare}
          class="flex-1 bg-transparent border border-bd-1 cursor-pointer p-3 rounded text-[12px] leading-none font-mono text-ink-2">cancel</button>
        <button
          onclick={handleConfirm}
          disabled={submitting || !shareSheet.feedUrl || (creatingGroup && !shareSheet.newGroupName.trim())}
          class="bg-cyan border-none cursor-pointer text-bg-0 font-semibold p-3 rounded flex-2 text-[12px] leading-none font-mono" style="opacity:{submitting||!shareSheet.feedUrl||(creatingGroup&&!shareSheet.newGroupName.trim())?'0.5':'1'};">
          {submitting ? 'adding…' : 'add feed'}
        </button>
      </div>
      {/if}
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>
