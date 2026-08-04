const active: Record<string, boolean> = $state({});
export function openOverlay(id: string) { active[id] = true; }
export function closeOverlay(id: string) { delete active[id]; }
export function anyOverlayOpen(): boolean {
  return Object.keys(active).length > 0;
}
