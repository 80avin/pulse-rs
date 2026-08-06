import type { FeedItem } from '../types';

// One shared item-action menu app-wide: ItemRow opens it, AppShell renders it
// once at the shell root, so it can't position inside a virtualized row or
// scroll with the list.
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
