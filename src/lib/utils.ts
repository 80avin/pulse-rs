export function sanitizeHtml(html: string): string {
  return html.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
             .replace(/<style\b[^<]*(?:(?!<\/style>)<[^<]*)*<\/style>/gi, '')
             .replace(/\s+on\w+="[^"]*"/gi, '')
             .replace(/\s+on\w+='[^']*'/gi, '');
}

export async function openExternal(url: string) {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch {
    window.open(url, '_blank', 'noopener');
  }
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
