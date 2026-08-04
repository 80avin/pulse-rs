export function longpress(
  node: HTMLElement,
  options: { delay?: number; threshold?: number; onLongpress: () => void }
) {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let startX = 0;
  let startY = 0;
  let fired = false;
  const delay = options.delay ?? 450;
  const threshold = options.threshold ?? 8;

  function onDown(clientX: number, clientY: number) {
    startX = clientX;
    startY = clientY;
    fired = false;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => { fired = true; options.onLongpress(); }, delay);
  }

  function onMove(clientX: number, clientY: number) {
    if (Math.abs(clientX - startX) > threshold || Math.abs(clientY - startY) > threshold) {
      if (timer) { clearTimeout(timer); timer = null; }
    }
  }

  function onEnd(e: Event) {
    if (timer) { clearTimeout(timer); timer = null; }
    if (fired) { e.preventDefault(); }
  }

  function touchStart(e: TouchEvent) {
    const t = e.touches[0];
    onDown(t.clientX, t.clientY);
  }

  function touchMove(e: TouchEvent) {
    const t = e.touches[0];
    onMove(t.clientX, t.clientY);
  }

  // Pointer handlers (for desktop mouse and GroupTabs)
  function pointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    onDown(e.clientX, e.clientY);
  }

  function pointerMove(e: PointerEvent) {
    onMove(e.clientX, e.clientY);
  }

  const supportsPointerEvents = 'PointerEvent' in window;
  if (supportsPointerEvents) {
    node.addEventListener('pointerdown', pointerDown);
    node.addEventListener('pointermove', pointerMove);
    node.addEventListener('pointerup', onEnd);
    node.addEventListener('pointercancel', onEnd);
  } else {
    node.addEventListener('touchstart', touchStart, { passive: true });
    node.addEventListener('touchmove', touchMove, { passive: true });
    node.addEventListener('touchend', onEnd);
    node.addEventListener('touchcancel', onEnd);
  }

  return {
    destroy() {
      if (timer) clearTimeout(timer);
      if (supportsPointerEvents) {
        node.removeEventListener("pointerdown", pointerDown);
        node.removeEventListener("pointermove", pointerMove);
        node.removeEventListener("pointerup", onEnd);
        node.removeEventListener("pointercancel", onEnd);
      } else {
        node.removeEventListener("touchstart", touchStart);
        node.removeEventListener("touchmove", touchMove);
        node.removeEventListener("touchend", onEnd);
        node.removeEventListener("touchcancel", onEnd);
      }
    },
  };
}
