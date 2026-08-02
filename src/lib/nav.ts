// Single source of truth for the narrow-shell navigation tabs. Both
// PulseBottomNav and AppShell must use this list so a tab id can never drift
// between the two (the earlier 'timeline' vs 'feed' mismatch).
export const TABS = ['feed', 'sources', 'search', 'saved', 'settings'] as const;
export type TabId = (typeof TABS)[number];

export function isTabId(v: string): v is TabId {
  return (TABS as readonly string[]).includes(v);
}
