import { describe, it, expect, vi, beforeEach } from 'vitest';

// The Tauri path is exercised here: IS_TAURI is captured at import time, so the
// window flag must be set before the store modules are imported (fresh instances
// via vi.resetModules per test). We stub window.__TAURI__.invoke directly —
// that is exactly what @tauri-apps/api/core's invoke() calls.

function defer<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((res) => { resolve = res; });
  return { promise, resolve };
}

function mkItem(id: string) {
  return {
    id, sourceId: 's1', sourceName: 'src', title: `title ${id}`, url: 'https://example.com',
    body: '', bodyHtml: null, externalUrl: null, author: null,
    publishedAt: '2026-01-01T00:00:00Z', read: false, saved: false, hidden: false,
    score: null, n: 0, tags: [], signal: 0, ogImage: null, note: null,
  };
}

type Page = { items: ReturnType<typeof mkItem>[]; nextCursor: null; counts: { total: number; unread: number; saved: number; signal: number } };

const COUNTS = { total: 1, unread: 1, saved: 0, signal: 0 };

describe('timeline request-epoch guard (category-flash bug R2)', () => {
  let tl: typeof import('./timeline.svelte');
  let data: typeof import('./data.svelte');
  let invoke: ReturnType<typeof vi.fn>;

  beforeEach(async () => {
    vi.resetModules();
    invoke = vi.fn((cmd: string, args: Record<string, unknown>) => {
      // initStore()'s bare { limit: 100 } call never resolves so it can't clobber test state.
      if (cmd === 'get_items_page' && args.limit === 100 && !('groupId' in args)) {
        return new Promise(() => {});
      }
      return Promise.resolve({ items: [], nextCursor: null, counts: COUNTS });
    });
    (globalThis as unknown as { window: unknown }).window = { __TAURI__: {}, __TAURI_INTERNALS__: { invoke } };
    data = await import('./data.svelte');
    tl = await import('./timeline.svelte');
    data.storeReady.loading = false;
  });

  it('a stale (older) response is discarded — last filter wins', async () => {
    const first = defer<Page>();
    const second = defer<Page>();
    invoke.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'get_items_page' && args.limit === 100 && !('groupId' in args)) {
        return new Promise(() => {});
      }
      if (args.groupId === 'g1') return first.promise;
      return second.promise;
    });

    const p1 = tl.applyFilter({ groupId: 'g1' });
    const p2 = tl.applyFilter({ groupId: 'g2' });

    // The g1 response arrives AFTER g2 was requested — it must be dropped.
    first.resolve({ items: [mkItem('from-g1')], nextCursor: null, counts: COUNTS });
    await p1;
    expect(tl.timelineFilter.groupId).toBe('g2');
    expect(data.items.length).toBe(0);

    second.resolve({ items: [mkItem('from-g2')], nextCursor: null, counts: COUNTS });
    await p2;
    expect(data.items.length).toBe(1);
    expect(data.items[0].id).toBe('from-g2');
  });

  it('non-stale responses still apply', async () => {
    invoke.mockImplementation((cmd: string, args: Record<string, unknown>) => {
      if (cmd === 'get_items_page' && args.limit === 100 && !('groupId' in args)) {
        return new Promise(() => {});
      }
      return Promise.resolve({ items: [mkItem('x')], nextCursor: null, counts: COUNTS });
    });

    await tl.applyFilter({ groupId: 'g1' });
    expect(data.items.length).toBe(1);
    expect(data.items[0].id).toBe('x');
  });
});
