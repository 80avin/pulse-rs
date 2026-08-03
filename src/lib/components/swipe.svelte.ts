export interface SwipeOptions {
  enabled?: boolean;
  blocked?: () => boolean;
  canSwipeLeft?: () => boolean;
  canSwipeRight?: () => boolean;
  onSwipeLeft?: () => void;
  onSwipeRight?: () => void;
}

export function swipeable(
  node: HTMLElement,
  options?: SwipeOptions | null
): { update(options?: SwipeOptions | null): void; destroy(): void } {
  let opts = options ?? null;
  let tracking = false;
  let startX = 0;
  let startY = 0;
  let currentX = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const enabled = () => opts?.enabled !== false;

  function reset() {
    node.style.transform = 'translateX(0px)';
    node.style.transition = 'none';
    currentX = 0;
  }

  function onTouchStart(e: TouchEvent) {
    if (!enabled()) return;
    if (opts?.blocked?.()) return;
    const t = e.touches[0];
    startX = t.clientX;
    startY = t.clientY;
    tracking = true;
    node.style.transition = 'none';
  }

  function onTouchMove(e: TouchEvent) {
    if (!enabled() || !tracking) return;
    if (opts?.blocked?.()) return;
    const dx = e.touches[0].clientX - startX;
    const dy = e.touches[0].clientY - startY;
    if (Math.abs(dy) > Math.abs(dx) || Math.abs(dx) < 8) return;
    currentX = dx * 0.5;
    node.style.transform = `translateX(${currentX}px)`;
  }

  function onTouchEnd() {
    if (!enabled() || !tracking) return;
    tracking = false;
    node.style.transition = '0.2s ease-out';
    const threshold = 60;
    const w = typeof window !== 'undefined' ? window.innerWidth : 400;
    if (currentX > threshold && opts?.canSwipeRight?.()) {
      node.style.transform = `translateX(${w}px)`;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        opts?.onSwipeRight?.();
        reset();
      }, 180);
    } else if (currentX < -threshold && opts?.canSwipeLeft?.()) {
      node.style.transform = `translateX(${-w}px)`;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        opts?.onSwipeLeft?.();
        reset();
      }, 180);
    } else {
      node.style.transform = 'translateX(0px)';
    }
  }

  if (enabled()) reset();

  node.addEventListener('touchstart', onTouchStart);
  node.addEventListener('touchmove', onTouchMove);
  node.addEventListener('touchend', onTouchEnd);

  return {
    update(next) {
      opts = next ?? null;
    },
    destroy() {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      node.removeEventListener('touchstart', onTouchStart);
      node.removeEventListener('touchmove', onTouchMove);
      node.removeEventListener('touchend', onTouchEnd);
    },
  };
}
