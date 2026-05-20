<script lang="ts">
  let { name, size = 14, color = 'currentColor', strokeWidth = 1.5, style = '' }: {
    name: string; size?: number; color?: string; strokeWidth?: number; style?: string;
  } = $props();

  const p = () => `fill:none;stroke:${color};stroke-width:${strokeWidth};stroke-linecap:round;stroke-linejoin:round`;
  const S = () => `width:${size}px;height:${size}px;display:block;${style}`;
</script>

{#if name === 'menu'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M3 6h18M3 12h18M3 18h18"/></svg>
{:else if name === 'search'}
  <svg viewBox="0 0 24 24" style={S()}><circle style={p()} cx="11" cy="11" r="7"/><path style={p()} d="M20 20l-4-4"/></svg>
{:else if name === 'filter'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M3 5h18l-7 9v6l-4-2v-4z"/></svg>
{:else if name === 'star'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M12 3l2.7 5.6 6.2.9-4.5 4.4 1.1 6.2L12 17.3 6.5 20l1.1-6.2L3.1 9.5l6.2-.9z"/></svg>
{:else if name === 'bookmark'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M6 3h12v18l-6-4-6 4z"/></svg>
{:else if name === 'eye'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle style={p()} cx="12" cy="12" r="3"/></svg>
{:else if name === 'ext'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M14 4h6v6M20 4L10 14M14 14v6H4V10h6"/></svg>
{:else if name === 'check'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M4 12l5 5L20 6"/></svg>
{:else if name === 'plus'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M12 5v14M5 12h14"/></svg>
{:else if name === 'x'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M6 6l12 12M18 6L6 18"/></svg>
{:else if name === 'cog'}
  <svg viewBox="0 0 24 24" style={S()}><circle style={p()} cx="12" cy="12" r="3"/><path style={p()} d="M19.4 15a1.7 1.7 0 00.3 1.9l.1.1a2 2 0 11-2.8 2.8l-.1-.1a1.7 1.7 0 00-1.9-.3 1.7 1.7 0 00-1 1.5V21a2 2 0 11-4 0v-.1a1.7 1.7 0 00-1.1-1.5 1.7 1.7 0 00-1.9.3l-.1.1a2 2 0 11-2.8-2.8l.1-.1a1.7 1.7 0 00.3-1.9 1.7 1.7 0 00-1.5-1H3a2 2 0 110-4h.1a1.7 1.7 0 001.5-1 1.7 1.7 0 00-.3-1.9l-.1-.1a2 2 0 112.8-2.8l.1.1a1.7 1.7 0 001.9.3H9a1.7 1.7 0 001-1.5V3a2 2 0 114 0v.1a1.7 1.7 0 001 1.5 1.7 1.7 0 001.9-.3l.1-.1a2 2 0 112.8 2.8l-.1.1a1.7 1.7 0 00-.3 1.9V9a1.7 1.7 0 001.5 1H21a2 2 0 110 4h-.1a1.7 1.7 0 00-1.5 1z"/></svg>
{:else if name === 'cpu'}
  <svg viewBox="0 0 24 24" style={S()}><rect style={p()} x="6" y="6" width="12" height="12" rx="1"/><path style={p()} d="M9 1v3M15 1v3M9 20v3M15 20v3M1 9h3M1 15h3M20 9h3M20 15h3"/><rect style={p()} x="9.5" y="9.5" width="5" height="5"/></svg>
{:else if name === 'bug'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M8 3l2 2h4l2-2M5 8l-2 1M5 8a4 4 0 014-4h6a4 4 0 014 4v6a7 7 0 11-14 0zM3 14h2M3 20l3-2M19 8l2 1M19 14h2M21 20l-3-2M12 8v13"/></svg>
{:else if name === 'dot'}
  <svg viewBox="0 0 24 24" style={S()}><circle cx="12" cy="12" r="4" fill={color}/></svg>
{:else if name === 'arrow-l'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M19 12H5M12 19l-7-7 7-7"/></svg>
{:else if name === 'arrow-r'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M5 12h14M12 5l7 7-7 7"/></svg>
{:else if name === 'chev-r'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M9 6l6 6-6 6"/></svg>
{:else if name === 'chev-dn'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M6 9l6 6 6-6"/></svg>
{:else if name === 'rss'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M4 11a9 9 0 019 9M4 4a16 16 0 0116 16"/><circle cx="5" cy="19" r="1.5" fill={color}/></svg>
{:else if name === 'reddit'}
  <svg viewBox="0 0 24 24" style={S()}><circle style={p()} cx="12" cy="13" r="8"/><circle cx="8.5" cy="13" r="1" fill={color}/><circle cx="15.5" cy="13" r="1" fill={color}/><path style={p()} d="M9 16c1 1 4.5 1 6 0M16 7a1.5 1.5 0 11.7 1.3"/></svg>
{:else if name === 'hn'}
  <svg viewBox="0 0 24 24" style={S()}><rect style={p()} x="3" y="3" width="18" height="18" rx="2"/><path style={p()} d="M8 8l4 5 4-5M12 13v4"/></svg>
{:else if name === 'sync'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M21 12a9 9 0 11-3-6.7M21 4v5h-5"/></svg>
{:else if name === 'database'}
  <svg viewBox="0 0 24 24" style={S()}><ellipse style={p()} cx="12" cy="5" rx="8" ry="3"/><path style={p()} d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/></svg>
{:else if name === 'activity'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>
{:else if name === 'info'}
  <svg viewBox="0 0 24 24" style={S()}><circle style={p()} cx="12" cy="12" r="9"/><path style={p()} d="M12 8h.01M11 12h1v5h1"/></svg>
{:else if name === 'home'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M3 12l9-9 9 9M5 10v10h5v-6h4v6h5V10"/></svg>
{:else if name === 'list'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M8 6h13M8 12h13M8 18h13"/><circle cx="3.5" cy="6" r="1" fill={color}/><circle cx="3.5" cy="12" r="1" fill={color}/><circle cx="3.5" cy="18" r="1" fill={color}/></svg>
{:else if name === 'image'}
  <svg viewBox="0 0 24 24" style={S()}><rect style={p()} x="3" y="3" width="18" height="18" rx="1"/><circle style={p()} cx="9" cy="9" r="2"/><path style={p()} d="M21 15l-5-5L5 21"/></svg>
{:else if name === 'video'}
  <svg viewBox="0 0 24 24" style={S()}><polygon points="6 4 20 12 6 20" fill={color}/></svg>
{:else if name === 'crosspost'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M7 17L17 7M10 7h7v7"/></svg>
{:else if name === 'link'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M10 14a4 4 0 005.7 0l3-3a4 4 0 00-5.7-5.7l-1 1M14 10a4 4 0 00-5.7 0l-3 3a4 4 0 005.7 5.7l1-1"/></svg>
{:else if name === 'edit'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M12 20h9M16.5 3.5a2.1 2.1 0 013 3L7 19l-4 1 1-4z"/></svg>
{:else if name === 'grip'}
  <svg viewBox="0 0 24 24" style={S()}><circle cx="9" cy="6" r="1.2" fill={color}/><circle cx="15" cy="6" r="1.2" fill={color}/><circle cx="9" cy="12" r="1.2" fill={color}/><circle cx="15" cy="12" r="1.2" fill={color}/><circle cx="9" cy="18" r="1.2" fill={color}/><circle cx="15" cy="18" r="1.2" fill={color}/></svg>
{:else if name === 'trash'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M3 6h18M8 6V4a1 1 0 011-1h6a1 1 0 011 1v2M6 6l1 14a2 2 0 002 2h6a2 2 0 002-2l1-14"/></svg>
{:else if name === 'eye-off'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M2 2l20 20M6.7 6.7C4.5 8 2.8 10 2 12c2 4.5 5.8 7 10 7 1.8 0 3.5-.5 5-1.4M10 10a3 3 0 004 4M17.4 17.4C19.5 16 21.2 14 22 12c-2-4.5-5.8-7-10-7-1.3 0-2.5.2-3.7.6"/></svg>
{:else if name === 'arrow-up'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M12 19V5M5 12l7-7 7 7"/></svg>
{:else if name === 'arrow-dn'}
  <svg viewBox="0 0 24 24" style={S()}><path style={p()} d="M12 5v14M5 12l7 7 7-7"/></svg>
{/if}
