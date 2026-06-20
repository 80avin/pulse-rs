# Pulse Frontend v2 — Design Spec

## Context

Pulse is a local-first, privacy-first feed reader (HN, Reddit, RSS) with on-device AI filtering. Built on Svelte 5 + SvelteKit SPA + Tauri 2. Current frontend (~5,600 lines, 41 files) suffers from: 100% inline styles with hardcoded dark-only theme, a 969-line monolithic store, 713-line DesktopShell, missing accessibility (touch targets <44px, no aria-labels, JS-managed focus), and no theming system.

## Goals

1. **Light/dark theme + accent color switching** — toggle from settings, persisted
2. **Density system** — dense/normal/roomy via CSS custom properties
3. **Accessible** — 44px touch targets, `:focus-visible` rings, aria-labels, hover feedback, skip-to-content
4. **Maintainable architecture** — store decomposed into hub-and-spoke modules, monster components split, consistent styling

## Non-Goals

- Full rewrite from scratch
- Router-based SPA (tabs/modals work fine)
- Component library adoption (Melt UI stays)
- Pull-to-refresh, `rem`-based font scaling, or any new features
- Removing all inline styles (hybrid; dynamic widths and computed colors stay inline)

---

## Architecture

### Store: Hub-and-Spoke Data Pattern

```
src/lib/stores/
├── data.svelte.ts      # items[], sources[], groups[], dbStats, initStore, ALL mutations
├── timeline.svelte.ts  # timelineFilter, pageCounts, loadMore, filter setters
├── ai.svelte.ts        # aiStatus, aiStats, models[], download/activate, retag
├── sync.svelte.ts      # syncState, doSync(), Tauri event listeners
├── search.svelte.ts    # searchItems(), FTS backend
└── settings.svelte.ts  # (existing, keep)
```

- `data.svelte.ts` is the single source of truth for reactive data arrays. All mutation functions (markRead, toggleSaved, hideItem, addSource, removeSource, etc.) live here. Contains cold-start `initStore()` retry loop and browser dev-mode mock data.
- Feature modules (timeline, ai, sync, search) import `data.svelte.ts`. No cross-imports between feature modules. No cycles.
- `settings.svelte.ts` must not import from `data.svelte.ts` (cycle prevention).
- All modules use Svelte 5 runes (`$state`, `$derived`).

### Styling: Tailwind CSS v4 + CSS Custom Properties

**`app.css`** — single source of truth for all design tokens via `@theme` directive with `light-dark()`.

**`tokens.ts`** — JS→CSS var bridge during migration:
```ts
export const T = { bg0: 'var(--color-bg-0)', /* ... */ };
```
Existing `{T.bg0}` inline styles resolve to CSS vars, which resolve to `light-dark()` values. Zero component changes for initial color migration.

**Theme toggle**: `<div style="color-scheme:{settings.theme}">` in layout. `light-dark()` picks colors automatically.

**Accent picker**: Injects `--user-accent` and `--user-accent-dim` CSS vars into `<svelte:head>`. Only cyan accent changes; tag colors and source spines are fixed per-kind.

**Density**: CSS custom properties on `:root[data-density]`:
```css
:root[data-density="dense"]  { --item-pad-y: 8px; }
:root[data-density="normal"] { --item-pad-y: 13px; }
:root[data-density="roomy"]  { --item-pad-y: 16px; }
```
Components use `var(--item-pad-y)` — no JS-derived density values.

**Tag colors**: Light-mode variants for all 30+ TAG_COLORS entries. Defined as CSS custom properties with `[data-theme]` selectors.

### Component Restructure

| Current | Target |
|---------|--------|
| `store.svelte.ts` (969) | `data.svelte.ts` (~500) + `timeline.svelte.ts` (~150) + `ai.svelte.ts` (~150) + `sync.svelte.ts` (~80) + `search.svelte.ts` (~40) |
| `DesktopShell.svelte` (713) | `DesktopShell.svelte` (~250) + `FilterPills.svelte` (~40) + `BottomTools.svelte` (~60) + `StatusBar.svelte` (~40) + `DragHandle.svelte` (~15) |
| `SourceExplorer.svelte` (504) | `SourceExplorer.svelte` (~350) + `SourceForm.svelte` (~80) |
| `SettingsPanelContent.svelte` (450) | `SettingsPanelContent.svelte` (~60) + `SettingsSection.svelte` (~20) + `ThemeSection.svelte` (~60) + `SyncSection.svelte` (~50) + `DataSection.svelte` (~50) + `AiSection.svelte` (~50) + `DiagnosticsSection.svelte` (~80) |
| `AiPanelContent.svelte` (282) | `AiPanelContent.svelte` (~250) + `TagBarChart.svelte` (~30) |
| `ReaderView.svelte` (191) | `ReaderView.svelte` (~140) + `NoteSheet.svelte` (~50) |
| `IconBtn.svelte` (21) | Rewrite: 44px min, required `aria-label`, `:focus-visible` ring |
| — | `SkipLink.svelte` (NEW, ~10 lines) |

### Accessibility

| Fix | How |
|-----|-----|
| Touch targets ≥44px | `min-h-[44px] min-w-[44px]` on all interactive elements |
| `aria-label` on icon buttons | Required prop on `IconBtn`; add to bare `<button>` instances |
| `:focus-visible` rings | `focus-visible:ring-2 focus-visible:ring-accent` via Tailwind |
| Hover feedback | `hover:bg-bg-2` or `hover:text-ink-0` on interactive surfaces |
| Color-only status | Add `aria-label` to `StatusDot` |
| Skip-to-content | New `SkipLink` component in `+layout.svelte` |

---

## Implementation Plan

### Phase 0: Tailwind Build Validation
Add `tailwindcss` + `@tailwindcss/vite`, configure Vite plugin, test with one component. If Tauri WebView breaks, revert 2 files.

### Phase 1: Token Migration
Convert `tokens.ts` to CSS var references. Add `@theme` + `light-dark()` to `app.css`. Add light-mode tag colors, density CSS vars, theme toggle in layout. Add `ThemeSection` to settings.

### Phase 2: Store Split
Extract `data.svelte.ts`, `timeline.svelte.ts`, `ai.svelte.ts`, `sync.svelte.ts`, `search.svelte.ts`. Use barrel re-exports for backward compatibility during transition, then remove.

### Phase 3: Component Extraction + Accessibility
Rewrite IconBtn. Add SkipLink. Extract FilterPills, BottomTools, StatusBar, DragHandle, SourceForm, NoteSheet, TagBarChart, SettingsSection + section components. Add aria-labels, focus-visible rings, hover states, skip-to-content.

### Phase 4: Tailwind Class Migration
Replace inline styles with Tailwind utilities for spacing, typography, borders, backgrounds. Keep inline for dynamic widths, computed colors, MeltUI elements.

### Phase 5: Cleanup
Remove `tokens.ts` (once fully migrated). Remove unused imports, dead code, consistency pass.

### Phase 6: Verify
`svelte-check`, `pnpm dev`, `pnpm build`, `detect_changes()`.

---

## Risks

| Risk | Mitigation |
|------|-----------|
| Tailwind v4 incompatible with Tauri WebView | Phase 0 validates first |
| `light-dark()` not supported in Android WebView | Test on device. Fallback: manual `data-theme` CSS |
| Import cycle: `data.svelte.ts` ↔ `settings.svelte.ts` | Verify during Phase 2 |
| TAG_COLORS light-mode contrast | Iterate if needed |
| MeltUI + Tailwind conflicts | Test one component early in Phase 3 |

## Dependencies Added

- `tailwindcss` ^4.x
- `@tailwindcss/vite` ^4.x

## Files Affected

~35 files modified, ~15 new files created.
