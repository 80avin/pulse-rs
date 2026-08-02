import type { FeedItem } from '../types';

// Single, shared item-action menu state. Only ONE menu exists app-wide:
// ItemRow triggers it, and AppShell renders it once at the shell root (outside
// any transformed/overflow ancestor), so it can't be positioned inside a
// virtualized row, scroll with the list, or appear multiple times.
export const itemMenu = $state<{
  current: {
    item: FeedItem;
    mode: 'popup' | 'sheet';
    x: number;
    y: number;
  } | null;
}>({ current: null });

export function openItemMenu(item: FeedItem, mode: 'popup' | 'sheet', x = 0, y = 0) {
  itemMenu.current = { item, mode, x, y };
}

export function closeItemMenu() {
  itemMenu.current = null;
}
