export interface Group {
  id: string;
  name: string;
  n: number;
}

export interface Source {
  id: string;
  kind: 'hn' | 'reddit' | 'rss';
  name: string;
  url?: string;
  host: string;
  items: number;
  unread: number;
  lastSync: string;
  status: 'ok' | 'stale' | 'error';
  latencyMs: number;
  group: string;
  failureStreak: number;
  hue?: number;
}

export interface Thumb {
  h: number;
  label?: string;
}

export interface FeedItem {
  id: string;
  src: string;
  kind: 'text' | 'link' | 'image' | 'video' | 'crosspost';
  title: string;
  author: string;
  age: string;
  score: number;
  n: number;
  tags: string[];
  userTags: string[];
  aiScore: number;
  read: boolean;
  saved: boolean;
  body: string;
  bodyHtml?: string;
  externalUrl?: string;
  url?: string;
  thumb?: Thumb;
  domain?: string;
  snippet?: string;
  dur?: string;
  crossFrom?: string;
  ogImage?: string | null;
  note?: string;
}

// The wire shape of a feed item from the Tauri backend (camelCase).
// Single source of truth — components no longer re-declare it (that caused
// silent drift when a DTO field was added).
export interface BackendItem {
  id: string; sourceId: string; sourceName: string; title: string; url: string;
  body: string; bodyHtml: string | null; externalUrl: string | null; author: string | null;
  publishedAt: string; read: boolean; saved: boolean; hidden: boolean;
  score: number | null; n: number; tags: string[]; signal: number;
  ogImage: string | null; note: string | null; userTags: string[];
}

export type Density = 'dense' | 'normal' | 'roomy';

