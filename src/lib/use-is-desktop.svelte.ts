const media = $state<{ isDesktop: boolean }>({ isDesktop: false });
if (typeof window !== 'undefined') {
  const mq = window.matchMedia('(min-width: 768px)');
  media.isDesktop = mq.matches;
  mq.addEventListener('change', (e) => { media.isDesktop = e.matches; });
}
export function isDesktop(): boolean {
  return media.isDesktop;
}
