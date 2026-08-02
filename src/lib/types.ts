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

export type Density = 'dense' | 'normal' | 'roomy';

