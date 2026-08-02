import DOMPurify from 'dompurify';

// Feed-supplied HTML (RSS content:encoded, HN/Reddit bodies) is attacker
// controlled. DOMPurify strips <script>/<style>/on* handlers, javascript: URLs,
// and other XSS vectors via an allowlist parser — a regex can't do this safely.
export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    // No embedded content or form controls from feed bodies.
    FORBID_TAGS: ['iframe', 'object', 'embed', 'form', 'input', 'button', 'select', 'textarea', 'video', 'audio', 'source', 'track'],
  });
}

export async function openExternal(url: string) {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch {
    window.open(url, '_blank', 'noopener');
  }
}

export async function shareItem(title: string, url?: string, body?: string): Promise<boolean> {
  const text = body ?? (url ? `${title}\n${url}` : title);
  const hasContent = !!url || !!text;

  // 1. Tauri plugin (Android/iOS native share sheet)
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('plugin:sharesheet|share_text', { text, options: { mime: 'text/plain' } });
      return true;
    } catch {
      // Plugin unavailable — fall through
    }
  }

  // 2. Web Share API
  try {
    if (typeof navigator !== 'undefined' && (navigator as any).share) {
      await (navigator as any).share({ title, text, url: url ?? '' });
      return true;
    }
  } catch {
    // User cancelled or unavailable
  }

  // 3. Clipboard fallback
  if (hasContent) {
    try { await navigator.clipboard.writeText(text); toast(body ? 'Content copied to clipboard' : 'URL copied to clipboard'); return true; } catch {}
  }
  return false;
}

function toast(message: string) {
  if (typeof document === 'undefined') return;
  const el = document.createElement('div');
  el.textContent = message;
  Object.assign(el.style, {
    position: 'fixed', bottom: '24px', left: '50%', transform: 'translateX(-50%)',
    background: '#0c1015', color: '#a7b2c2', border: '1px solid #2c3645',
    padding: '8px 16px', borderRadius: '4px', fontFamily: 'monospace', fontSize: '12px',
    zIndex: '9999', opacity: '0', transition: 'opacity 0.2s ease',
    pointerEvents: 'none',
  });
  document.body.appendChild(el);
  requestAnimationFrame(() => { el.style.opacity = '1'; });
  setTimeout(() => {
    el.style.opacity = '0';
    setTimeout(() => el.remove(), 200);
  }, 2000);
}

// Tag → evidence snippets for the AI explain popover
export const TAG_EVIDENCE: Record<string, string[]> = {
  technical:    ['"NNAPI delegate"', '"WAL-mode writes"', '"Rust + Svelte"'],
  research:     ['"per-CPU page lists"', '"defragmentation"', '"benchmarks"'],
  tutorial:     ['"a practical guide"', '"step-by-step"', '"how I debug"'],
  news:         ['"announced"', '"released"', '"reports"'],
  meme:         ['"with cats"', '"explained in 27min"'],
  ragebait:     ['"why I left"', '"is dead"'],
  'low-effort': ['"[screenshot.png]"', 'title=URL'],
  screenshot:   ['"[screenshot.png]"', 'attached image'],
  clickbait:    ['"rumored to outperform"', 'unverified source'],
  release:      ['"released"', '"v7.6"', '"now available"'],
  deepdive:     ['>2000 words', 'cites 4 papers'],
  'ai/ml':      ['"model"', '"inference"', '"benchmark"'],
};
