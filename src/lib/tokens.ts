export const T = {
  bg0: 'var(--color-bg-0)', bg1: 'var(--color-bg-1)', bg2: 'var(--color-bg-2)', bg3: 'var(--color-bg-3)',
  bd0: 'var(--color-bd-0)', bd1: 'var(--color-bd-1)', bd2: 'var(--color-bd-2)',
  ink0: 'var(--color-ink-0)', ink1: 'var(--color-ink-1)', ink2: 'var(--color-ink-2)', ink3: 'var(--color-ink-3)', ink4: 'var(--color-ink-4)',
  cyan: 'var(--color-cyan)', cyanDim: 'var(--color-cyan-dim)',
  amber: 'var(--color-amber)', amberDim: 'var(--color-amber-dim)',
  green: 'var(--color-green)', greenDim: 'var(--color-green-dim)',
  red: 'var(--color-red)', redDim: 'var(--color-red-dim)',
  violet: 'var(--color-violet)',
  orange: 'var(--color-orange)',
  pink: 'var(--color-pink)',
  mono: 'var(--font-mono)',
  sans: 'var(--font-sans)',
};

export const TAG_COLORS: Record<string, { fg: string; bg: string; bd: string }> = {
  technical:    { fg: 'var(--tag-technical-fg)',    bg: 'var(--tag-technical-bg)',    bd: 'var(--tag-technical-bd)' },
  research:     { fg: 'var(--tag-research-fg)',     bg: 'var(--tag-research-bg)',     bd: 'var(--tag-research-bd)' },
  tutorial:     { fg: 'var(--tag-tutorial-fg)',     bg: 'var(--tag-tutorial-bg)',     bd: 'var(--tag-tutorial-bd)' },
  news:         { fg: 'var(--tag-news-fg)',         bg: 'var(--tag-news-bg)',         bd: 'var(--tag-news-bd)' },
  'ai-ml':      { fg: 'var(--tag-ai-ml-fg)',       bg: 'var(--tag-ai-ml-bg)',        bd: 'var(--tag-ai-ml-bd)' },
  security:     { fg: 'var(--tag-security-fg)',     bg: 'var(--tag-security-bg)',     bd: 'var(--tag-security-bd)' },
  privacy:      { fg: 'var(--tag-privacy-fg)',      bg: 'var(--tag-privacy-bg)',      bd: 'var(--tag-privacy-bd)' },
  policy:       { fg: 'var(--tag-policy-fg)',       bg: 'var(--tag-policy-bg)',       bd: 'var(--tag-policy-bd)' },
  science:      { fg: 'var(--tag-science-fg)',      bg: 'var(--tag-science-bg)',      bd: 'var(--tag-science-bd)' },
  clickbait:    { fg: 'var(--tag-clickbait-fg)',    bg: 'var(--tag-clickbait-bg)',    bd: 'var(--tag-clickbait-bd)' },
  'show-hn':    { fg: 'var(--tag-show-hn-fg)',      bg: 'var(--tag-show-hn-bg)',      bd: 'var(--tag-show-hn-bd)' },
  'ask-hn':     { fg: 'var(--tag-ask-hn-fg)',       bg: 'var(--tag-ask-hn-bg)',       bd: 'var(--tag-ask-hn-bd)' },
  'job-posting':{ fg: 'var(--tag-job-posting-fg)',  bg: 'var(--tag-job-posting-bg)',  bd: 'var(--tag-job-posting-bd)' },
  paywall:      { fg: 'var(--tag-paywall-fg)',      bg: 'var(--tag-paywall-bg)',      bd: 'var(--tag-paywall-bd)' },
  video:        { fg: 'var(--tag-video-fg)',        bg: 'var(--tag-video-bg)',        bd: 'var(--tag-video-bd)' },
  meme:         { fg: 'var(--tag-meme-fg)',         bg: 'var(--tag-meme-bg)',         bd: 'var(--tag-meme-bd)' },
  screenshot:   { fg: 'var(--tag-screenshot-fg)',   bg: 'var(--tag-screenshot-bg)',   bd: 'var(--tag-screenshot-bd)' },
  'photo-share':{ fg: 'var(--tag-photo-share-fg)',  bg: 'var(--tag-photo-share-bg)',  bd: 'var(--tag-photo-share-bd)' },
  civic:        { fg: 'var(--tag-civic-fg)',        bg: 'var(--tag-civic-bg)',        bd: 'var(--tag-civic-bd)' },
  'local-rec':  { fg: 'var(--tag-local-rec-fg)',    bg: 'var(--tag-local-rec-bg)',    bd: 'var(--tag-local-rec-bd)' },
  culture:      { fg: 'var(--tag-culture-fg)',      bg: 'var(--tag-culture-bg)',      bd: 'var(--tag-culture-bd)' },
  marketplace:  { fg: 'var(--tag-marketplace-fg)',  bg: 'var(--tag-marketplace-bg)',  bd: 'var(--tag-marketplace-bd)' },
  'low-effort': { fg: 'var(--tag-low-effort-fg)',   bg: 'var(--tag-low-effort-bg)',   bd: 'var(--tag-low-effort-bd)' },
  'no-context': { fg: 'var(--tag-no-context-fg)',   bg: 'var(--tag-no-context-bg)',   bd: 'var(--tag-no-context-bd)' },
  inappropriate:{ fg: 'var(--tag-inappropriate-fg)', bg: 'var(--tag-inappropriate-bg)', bd: 'var(--tag-inappropriate-bd)' },
  noise:        { fg: 'var(--tag-noise-fg)',        bg: 'var(--tag-noise-bg)',        bd: 'var(--tag-noise-bd)' },
};

export const SOURCE_KIND: Record<string, { glyph: string; color: string; accent: string; spine: string }> = {
  rss:    { glyph: 'rss', color: 'var(--color-amber)',  accent: 'var(--color-amber)',  spine: 'var(--color-spine-rss)' },
  hn:     { glyph: 'hn',  color: 'var(--color-orange)', accent: 'var(--color-orange)', spine: 'var(--color-spine-hn)' },
  reddit: { glyph: 'r/',  color: 'var(--color-cyan)',   accent: 'var(--color-cyan)',   spine: 'var(--color-spine-reddit)' },
};

export function sourcePillLabel(name: string): string {
  const parts = name.split(/[\s/]+/).filter(Boolean);
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
  return name.slice(0, 2).toUpperCase();
}

export function sourcePillHue(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = ((h << 5) - h + id.charCodeAt(i)) | 0;
  return Math.abs(h) % 360;
}
