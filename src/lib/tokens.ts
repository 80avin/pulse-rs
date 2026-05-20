export const T = {
  bg0: '#07090c', bg1: '#0c1015', bg2: '#11161e', bg3: '#171d27',
  bd0: '#1a212c', bd1: '#222b38', bd2: '#2c3645',
  ink0: '#e6ebf2', ink1: '#a7b2c2', ink2: '#6c7686', ink3: '#454d5b', ink4: '#2a313c',
  cyan: '#4ecdd6', cyanDim: '#2a7a82',
  amber: '#e6b450', amberDim: '#7a5e2a',
  green: '#6bd896', greenDim: '#2f6a44',
  red: '#e26b6b', redDim: '#7a2f2f',
  violet: '#b48ce6',
  orange: '#e69056',
  pink: '#e078b8',
  mono: '"JetBrains Mono", "Berkeley Mono", "Fira Code", ui-monospace, monospace',
  sans: '"Inter", ui-sans-serif, system-ui, -apple-system, sans-serif',
};

export const TAG_COLORS: Record<string, { fg: string; bg: string; bd: string }> = {
  // ── Topic tags ──────────────────────────────────────────────────────────────
  technical:    { fg: T.cyan,   bg: 'rgba(78,205,214,0.10)',  bd: 'rgba(78,205,214,0.30)'  },
  research:     { fg: T.violet, bg: 'rgba(180,140,230,0.10)', bd: 'rgba(180,140,230,0.30)' },
  tutorial:     { fg: T.green,  bg: 'rgba(107,216,150,0.10)', bd: 'rgba(107,216,150,0.30)' },
  news:         { fg: T.amber,  bg: 'rgba(230,180,80,0.10)',  bd: 'rgba(230,180,80,0.30)'  },
  'ai-ml':      { fg: T.amber,  bg: 'rgba(230,180,80,0.10)',  bd: 'rgba(230,180,80,0.30)'  },
  security:     { fg: T.red,    bg: 'rgba(226,107,107,0.10)', bd: 'rgba(226,107,107,0.30)' },
  privacy:      { fg: T.violet, bg: 'rgba(180,140,230,0.08)', bd: 'rgba(180,140,230,0.25)' },
  policy:       { fg: T.ink1,   bg: 'rgba(167,178,194,0.08)', bd: 'rgba(167,178,194,0.25)' },
  science:      { fg: T.cyan,   bg: 'rgba(78,205,214,0.08)',  bd: 'rgba(78,205,214,0.25)'  },
  clickbait:    { fg: T.orange, bg: 'rgba(230,144,86,0.10)',  bd: 'rgba(230,144,86,0.30)'  },
  // ── HN / feed type tags ─────────────────────────────────────────────────────
  'show-hn':    { fg: T.cyan,   bg: 'rgba(78,205,214,0.10)',  bd: 'rgba(78,205,214,0.30)'  },
  'ask-hn':     { fg: T.green,  bg: 'rgba(107,216,150,0.10)', bd: 'rgba(107,216,150,0.30)' },
  'job-posting':{ fg: T.amber,  bg: 'rgba(230,180,80,0.10)',  bd: 'rgba(230,180,80,0.30)'  },
  paywall:      { fg: T.orange, bg: 'rgba(230,144,86,0.08)',  bd: 'rgba(230,144,86,0.25)'  },
  video:        { fg: T.pink,   bg: 'rgba(224,120,184,0.10)', bd: 'rgba(224,120,184,0.30)' },
  // ── Image/visual tags ───────────────────────────────────────────────────────
  meme:         { fg: T.pink,   bg: 'rgba(224,120,184,0.10)', bd: 'rgba(224,120,184,0.30)' },
  screenshot:   { fg: T.ink1,   bg: 'rgba(167,178,194,0.08)', bd: 'rgba(167,178,194,0.25)' },
  'photo-share':{ fg: T.green,  bg: 'rgba(107,216,150,0.08)', bd: 'rgba(107,216,150,0.25)' },
  // ── Local community tags  ───────────────────────────────────────────────────
  civic:        { fg: T.red,    bg: 'rgba(226,107,107,0.10)', bd: 'rgba(226,107,107,0.30)' },
  'local-rec':  { fg: T.green,  bg: 'rgba(107,216,150,0.10)', bd: 'rgba(107,216,150,0.30)' },
  culture:      { fg: T.violet, bg: 'rgba(180,140,230,0.10)', bd: 'rgba(180,140,230,0.30)' },
  marketplace:  { fg: T.amber,  bg: 'rgba(230,180,80,0.10)',  bd: 'rgba(230,180,80,0.30)'  },
  // ── Quality / signal tags ───────────────────────────────────────────────────
  'low-effort': { fg: T.ink2,   bg: 'rgba(108,118,134,0.10)', bd: 'rgba(108,118,134,0.30)' },
  'no-context': { fg: T.ink2,   bg: 'rgba(108,118,134,0.10)', bd: 'rgba(108,118,134,0.30)' },
  inappropriate:{ fg: T.red,    bg: 'rgba(226,107,107,0.10)', bd: 'rgba(226,107,107,0.30)' },
  noise:        { fg: T.ink3,   bg: 'rgba(69,77,91,0.10)',    bd: 'rgba(69,77,91,0.30)'    },
};

export const SOURCE_KIND: Record<string, { glyph: string; color: string }> = {
  rss:    { glyph: 'rss', color: T.amber  },
  hn:     { glyph: 'hn',  color: T.orange },
  reddit: { glyph: 'r/',  color: T.cyan   },
};
