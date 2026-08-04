// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { longpress } from './longpress.svelte';

describe('longpress action', () => {
  let node: HTMLDivElement;
  let onLongpress = vi.fn<() => void>();

  beforeEach(() => {
    node = document.createElement('div');
    onLongpress = vi.fn();
  });

  it('fires on primary-button hold', async () => {
    const act = longpress(node, { delay: 10, onLongpress });
    node.dispatchEvent(new PointerEvent('pointerdown', { button: 0, clientX: 0, clientY: 0 }));
    await new Promise((r) => setTimeout(r, 30));
    expect(onLongpress).toHaveBeenCalledTimes(1);
    act.destroy();
  });

  it('ignores right-button hold (context menu must not long-press)', async () => {
    const act = longpress(node, { delay: 10, onLongpress });
    node.dispatchEvent(new PointerEvent('pointerdown', { button: 2, clientX: 0, clientY: 0 }));
    await new Promise((r) => setTimeout(r, 30));
    expect(onLongpress).not.toHaveBeenCalled();
    act.destroy();
  });

  it('ignores middle-button hold', async () => {
    const act = longpress(node, { delay: 10, onLongpress });
    node.dispatchEvent(new PointerEvent('pointerdown', { button: 1, clientX: 0, clientY: 0 }));
    await new Promise((r) => setTimeout(r, 30));
    expect(onLongpress).not.toHaveBeenCalled();
    act.destroy();
  });
});
