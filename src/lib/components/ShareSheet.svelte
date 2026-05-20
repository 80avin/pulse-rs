<script lang="ts">
  import { T } from '$lib/tokens';
  import { groups } from '$lib/store.svelte';
  import { shareSheet, dismissShare, confirmShare } from '$lib/share.svelte';

  let submitting = $state(false);

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

<div
  role="button"
  tabindex="-1"
  onclick={dismissShare}
  onkeydown={(e) => e.key === 'Escape' && dismissShare()}
  style="position:fixed;inset:0;background:rgba(0,0,0,0.6);z-index:300;display:flex;align-items:flex-end;"
>
  <div
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={() => {}}
    style="width:100%;background:{T.bg2};border-radius:16px 16px 0 0;padding:20px 16px max(20px,env(safe-area-inset-bottom));display:flex;flex-direction:column;gap:14px;"
  >
    <!-- Header -->
    <div style="display:flex;align-items:center;gap:8px;">
      <span style="flex:1;font:600 12px/1 {T.mono};color:{T.ink0};letter-spacing:0.3px;">ADD FEED FROM SHARE</span>
      <button onclick={dismissShare} style="background:transparent;border:none;cursor:pointer;padding:4px;color:{T.ink2};font-size:16px;">&#x2715;</button>
    </div>

    {#if shareSheet.loading}
      <div style="text-align:center;padding:20px;font:11px/1 {T.mono};color:{T.ink3};">detecting feed…</div>
    {:else}
      <!-- HN notice -->
      {#if shareSheet.candidate?.isHn}
        <div style="padding:8px 10px;background:{T.bg1};border-left:3px solid {T.amber};font:10px/1.5 {T.mono};color:{T.amber};">
          HN has one global feed — this subscribes to the front page.
        </div>
      {/if}

      <!-- Network / detection error -->
      {#if shareSheet.error}
        <div style="font:10px/1.4 {T.mono};color:{T.red};">Could not reach this URL. Check connectivity or paste the feed URL directly.</div>
      {/if}

      <!-- Feed name -->
      <div style="display:flex;flex-direction:column;gap:5px;">
        <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">NAME</label>
        <input bind:value={shareSheet.name} placeholder="Feed name"
          style="padding:10px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:12px/1 {T.mono};color:{T.ink0};width:100%;box-sizing:border-box;" />
      </div>

      <!-- Feed URL + no-feed warning -->
      <div style="display:flex;flex-direction:column;gap:5px;">
        <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">FEED URL</label>
        <input bind:value={shareSheet.feedUrl} placeholder="https://…"
          style="padding:10px;background:{T.bg0};border:1px solid {noFeedFound ? T.red : T.bd1};border-radius:3px;font:11px/1 {T.mono};color:{T.ink0};width:100%;box-sizing:border-box;" />
        {#if noFeedFound}
          <span style="font:9px/1.4 {T.mono};color:{T.red};">No feed source found at this URL. Paste a direct RSS/Atom URL above if you have one.</span>
        {/if}
      </div>

      <!-- Alternate feed picker -->
      {#if (shareSheet.candidate?.candidates?.length ?? 0) > 1}
        <div style="display:flex;flex-direction:column;gap:4px;">
          <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">ALTERNATE FEEDS</label>
          {#each shareSheet.candidate!.candidates as c}
            <button onclick={() => { shareSheet.feedUrl = c.url; if (c.title) shareSheet.name = c.title; }}
              style="text-align:left;padding:8px;background:{shareSheet.feedUrl===c.url?T.bg3:T.bg0};border:1px solid {shareSheet.feedUrl===c.url?T.cyan:T.bd1};border-radius:3px;font:10px/1.4 {T.mono};color:{T.ink1};cursor:pointer;">
              {c.title ?? c.url}
              <span style="display:block;color:{T.ink3};overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">{c.url}</span>
            </button>
          {/each}
        </div>
      {/if}

      <!-- Type + Group row -->
      <div style="display:flex;gap:8px;">
        <!-- Type toggle -->
        <div style="flex:1;display:flex;flex-direction:column;gap:5px;">
          <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">TYPE</label>
          <div style="display:flex;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;padding:2px;gap:2px;">
            {#each (['rss','hn','reddit'] as const) as k}
              <button onclick={() => shareSheet.kind = k}
                style="flex:1;padding:6px 2px;border:none;border-radius:2px;cursor:pointer;font:9px/1 {T.mono};text-transform:uppercase;background:{shareSheet.kind===k?T.bg3:'transparent'};color:{shareSheet.kind===k?T.cyan:T.ink2};">
                {k}
              </button>
            {/each}
          </div>
        </div>

        <!-- Group selector -->
        <div style="flex:1;display:flex;flex-direction:column;gap:5px;">
          <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">GROUP</label>
          <select bind:value={shareSheet.group}
            style="width:100%;padding:8px;background:{T.bg0};border:1px solid {T.bd1};border-radius:3px;font:11px/1 {T.mono};color:{T.ink0};">
            {#each groups as g}<option value={g.id}>{g.name}</option>{/each}
            <option value="__new__">+ new group</option>
          </select>
        </div>
      </div>

      <!-- New group name input (shown when __new__ selected) -->
      {#if creatingGroup}
        <div style="display:flex;flex-direction:column;gap:5px;">
          <label style="font:9px/1 {T.mono};color:{T.ink3};letter-spacing:0.5px;">NEW GROUP NAME</label>
          <input
            bind:value={shareSheet.newGroupName}
            placeholder="e.g. Tech, Local, Work…"
            autofocus
            style="padding:10px;background:{T.bg0};border:1px solid {T.cyan};border-radius:3px;font:12px/1 {T.mono};color:{T.ink0};width:100%;box-sizing:border-box;"
          />
        </div>
      {/if}

      <!-- Action buttons -->
      <div style="display:flex;gap:8px;margin-top:4px;">
        <button onclick={dismissShare}
          style="flex:1;padding:12px;background:transparent;border:1px solid {T.bd1};border-radius:4px;font:12px/1 {T.mono};color:{T.ink2};cursor:pointer;">cancel</button>
        <button
          onclick={handleConfirm}
          disabled={submitting || !shareSheet.feedUrl || (creatingGroup && !shareSheet.newGroupName.trim())}
          style="flex:2;padding:12px;background:{T.cyan};border:none;border-radius:4px;font:600 12px/1 {T.mono};color:{T.bg0};cursor:pointer;opacity:{submitting||!shareSheet.feedUrl||(creatingGroup&&!shareSheet.newGroupName.trim())?'0.5':'1'};">
          {submitting ? 'adding…' : 'add feed'}
        </button>
      </div>
    {/if}
  </div>
</div>
