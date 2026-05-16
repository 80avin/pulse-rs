# Pulse — UI Adaptation Strategy

## Overview

This document covers how the headless `pulse-core` library maps to the Tauri+Svelte UI layer in Phase 3. The UI is a thin presentation layer over the core engine — it does not implement any business logic independently.

Key constraint: **The UI layer must not diverge from the CLI in terms of capability.** Anything the CLI can do, the UI must be able to do. The core API is the single source of truth.

## Core-to-UI Boundary

### Tauri Commands

The Tauri shell (`src-tauri`) exposes `pulse-core` functionality as Tauri commands. Each command is a thin async wrapper:

```rust
// src-tauri/src/lib.rs

#[tauri::command]
async fn get_timeline(
    state: tauri::State<'_, PulseState>,
    group_id: Option<String>,
    limit: usize,
    cursor: Option<TimelineCursor>,
    filter: TimelineFilter,
) -> Result<TimelinePage, String> {
    state.core.timeline().get_page(group_id.as_deref(), limit, cursor, filter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_item_read(
    state: tauri::State<'_, PulseState>,
    item_id: String,
) -> Result<(), String> {
    state.core.items().mark_read(&item_id)
        .await
        .map_err(|e| e.to_string())
}
```

The `PulseState` struct holds the `Arc<PulseCore>` and is initialized in `setup()`.

### Event System

The Tauri shell emits events to the frontend when background operations complete:

```rust
// Events emitted from Tauri to Svelte frontend:
"pulse://sync-complete"      // { feed_id, new_item_count, error? }
"pulse://sync-started"       // { feed_id }
"pulse://tagging-complete"   // { item_id, tags: [...] }
"pulse://model-download-progress" // { model_id, progress: 0.0-1.0 }
"pulse://feed-health-changed" // { feed_id, health_status }
```

The Svelte frontend listens to these events and updates reactive state accordingly. This avoids polling and keeps the UI in sync with background operations without any timer-based refresh loops.

## Svelte State Architecture

### Feature-Oriented Stores

State is organized by feature, not by data type. No single global store. Each feature owns its slice of state.

```
src/lib/
├── feeds/
│   ├── feedStore.ts         # feed list, health status
│   ├── groupStore.ts        # group list and active group
│   └── feedService.ts       # Tauri command calls for feeds
├── timeline/
│   ├── timelineStore.ts     # items, pagination cursor, loading state
│   ├── filterStore.ts       # active filters (read/unread/saved/tags)
│   └── timelineService.ts
├── reader/
│   ├── readerStore.ts       # currently open item, scroll position
│   └── readerService.ts
├── search/
│   ├── searchStore.ts       # query, results, loading
│   └── searchService.ts
├── ai/
│   ├── aiStore.ts           # active model, tag rules, tagging queue depth
│   └── aiService.ts
└── sync/
    ├── syncStore.ts         # sync status per feed, last sync times
    └── syncService.ts       # Tauri event listeners for sync events
```

### Store Pattern

Svelte 5 uses `$state` runes. Each feature store exports reactive state and actions:

```typescript
// src/lib/timeline/timelineStore.ts
import { invoke } from '@tauri-apps/api/core';

export const timelineItems = $state<FeedItemView[]>([]);
export const isLoading = $state(false);
export const cursor = $state<TimelineCursor | null>(null);
export const hasMore = $state(true);
export const activeFilter = $state<TimelineFilter>({ showRead: false, showHidden: false });

export async function loadMore() {
    if (isLoading || !hasMore) return;
    isLoading = true;
    try {
        const page = await invoke<TimelinePage>('get_timeline', {
            limit: 50,
            cursor: cursor,
            filter: activeFilter,
        });
        timelineItems.push(...page.items);
        cursor = page.next_cursor;
        hasMore = page.has_more;
    } finally {
        isLoading = false;
    }
}
```

No shared global state bus. Components import only the stores they need.

### Sync Event Integration

The `syncService.ts` sets up Tauri event listeners once at app startup:

```typescript
// src/lib/sync/syncService.ts
import { listen } from '@tauri-apps/api/event';
import { invalidateTimeline } from '../timeline/timelineStore';
import { updateFeedHealth } from '../feeds/feedStore';

export async function initSyncListeners() {
    await listen('pulse://sync-complete', (event) => {
        const { feed_id, new_item_count } = event.payload;
        if (new_item_count > 0) {
            invalidateTimeline();  // mark cache stale → lazy reload on next scroll
        }
        updateFeedHealth(feed_id);
    });
}
```

## Component Architecture

### Route Structure (SvelteKit)

```
src/routes/
├── +layout.svelte         # Shell: nav, status bar, sync indicator
├── +layout.ts             # Load feeds, groups on startup
├── +page.svelte           # Redirect to /timeline
├── timeline/
│   ├── +page.svelte       # Timeline view
│   └── [item_id]/
│       └── +page.svelte   # Item reader view
├── feeds/
│   ├── +page.svelte       # Feed management
│   └── [feed_id]/
│       └── +page.svelte   # Single feed view
├── search/
│   └── +page.svelte       # Search results
├── settings/
│   └── +page.svelte       # Settings (AI models, rules, preferences)
└── diagnostics/
    └── +page.svelte       # Diagnostics panel
```

### Key Components

```
src/lib/components/
├── timeline/
│   ├── TimelineList.svelte      # Virtualized list of items
│   ├── TimelineItem.svelte      # Single item row (compact)
│   └── TimelineFilters.svelte   # Filter controls
├── reader/
│   ├── ReaderPane.svelte        # Article reader view
│   └── ReaderMeta.svelte        # Title, date, tags, score
├── feeds/
│   ├── FeedList.svelte
│   ├── FeedRow.svelte           # Feed with health indicator
│   └── FeedHealthBadge.svelte
├── ai/
│   ├── TagBadge.svelte          # Tag with confidence + explanation tooltip
│   ├── TagList.svelte
│   └── ModelSelector.svelte
├── shared/
│   ├── SyncStatus.svelte        # Top bar sync indicator
│   ├── GroupTabs.svelte         # Group navigation
│   └── SearchBar.svelte
└── layout/
    ├── Sidebar.svelte           # Left navigation (desktop/tablet)
    └── BottomNav.svelte         # Bottom navigation (phone)
```

## Design Reference Interpretation

The prototype design (referenced in idea.md) provides the reference direction. This section describes how to interpret it for Pulse's goals.

### What to Adopt

**Information density**: The prototype's compact list view is the right model. Items should show in ~40-48px rows on mobile, less on desktop. No card-based layout.

**Navigation structure**: Side navigation on desktop/tablet; bottom navigation on mobile. The group tabs concept maps directly to `feed_groups`.

**Reader integration**: The reader should feel attached to the list, not a separate full-screen app. Dual-pane on tablet/desktop; slide-over or route-push on phone.

**Minimal chrome**: The prototype shows minimal UI chrome. Status information (sync state, unread count) should be in a single status bar, not scattered across multiple UI elements.

### What to Adapt

**Typography scale**: Reduce font sizes by 1-2 steps from typical mobile apps. Power users want density, not readability-optimized font sizes. Minimum 13sp body text (still accessible).

**Touch targets**: The prototype may show elements too small for reliable touch. Every interactive element must meet 44x44dp minimum touch target. Achieve this with padding, not size.

**Color palette**: Do not hardcode a specific color scheme. Support system light/dark mode. Accent color should be a single neutral (blue or green), not multiple competing colors.

**Whitespace**: Reduce padding by 30-40% from typical Material/iOS defaults. Dense lists need tight spacing.

### What Not to Do

- No card shadows or elevation system
- No hero images for article previews
- No gradient backgrounds
- No floating action button
- No bottom sheets for simple actions (use inline controls instead)
- No onboarding splash screen
- No empty state illustrations (text only)

## Layout Strategy

### Phone (< 600dp width): Single Column

```
┌──────────────────────────┐
│ [≡] Pulse  ⬛ [sync] [⚙]│  ← Top bar: menu, title, sync dot, settings
├──────────────────────────┤
│ [All] [tech] [ai] [news] │  ← Horizontal scrollable group tabs
├──────────────────────────┤
│ ● 2h ▲2431 r/rust        │
│   Announcing Rust 2025   │
├──────────────────────────┤
│ ● 3h ★ 847 Pragmatic     │
│   On Being a Staff...    │
├──────────────────────────┤
│   4h ▲ 156 r/prog        │
│   Why Rust's borrow ch...│
├──────────────────────────┤
│    (scrollable list)     │
├──────────────────────────┤
│ [🏠 Timeline] [🔍] [📚] │  ← Bottom nav: timeline, search, saved
└──────────────────────────┘
```

Tapping an item → push to reader route (full-screen reader).

### Tablet (600-1200dp width): Dual Pane

```
┌──────────────────────────────────────────────────────────────────────┐
│ [≡] Pulse       [All][tech][ai][news]    ⬛ syncing  [⚙] settings   │
├───────────────────────┬──────────────────────────────────────────────┤
│ ● 2h ▲2431 r/rust     │ On Being a Staff Engineer                    │
│   Announcing Rust...  │ The Pragmatic Engineer · 2026-05-14 · 12min  │
│                       │ Tags: technical  research                    │
│ ● 3h ★ 847 Pragmatic  │ ─────────────────────────────────────────── │
│   On Being a Staff... │ The transition from senior engineer to staff  │
│                       │ engineer is one of the most underappreciated │
│   4h ▲ 156 r/prog     │ career shifts in software...                 │
│   Why Rust's borrow.. │                                               │
│                       │                                               │
│   (item list)         │   (reader — auto-updates on selection)       │
├───────────────────────┴──────────────────────────────────────────────┤
│ [Timeline]  [Feeds]  [Search]  [Saved]                               │
└──────────────────────────────────────────────────────────────────────┘
```

### Desktop (> 1200dp width): Three-column optional

```
┌──────────┬───────────────────────────┬──────────────────────────────┐
│ Groups   │ Item List                 │ Reader                       │
│          │                           │                              │
│ All      │ ● 2h ▲2431 r/rust         │ On Being a Staff Engineer    │
│ tech     │   Announcing Rust 2025    │ ...                          │
│ ai       │ ● 3h ★ 847 Pragmatic      │                              │
│ news     │   On Being a Staff...     │                              │
│          │ (list)                    │ (article body)               │
└──────────┴───────────────────────────┴──────────────────────────────┘
```

## Android-Specific Considerations

### Back Navigation

Android's back button must be handled explicitly. Routes in order: item reader → timeline → app exit. The Tauri app handles this via the hardware back button listener.

### Keyboard Support

Some Android users use hardware keyboards (especially tablet users). Map the same keybindings as the TUI (`j/k` for navigation, `r` for read, etc.) via keyboard event listeners.

### System Insets

Handle system UI insets (status bar, navigation bar, notch). Tauri 2 on Android provides safe area insets. Apply as CSS env() variables (`padding: env(safe-area-inset-top)`).

### Font Scaling

Respect system font scaling. Do not use fixed pixel sizes for text. Use CSS `rem` units. Test at 150% system font scale — layout must not break.

### Refresh Gesture

Standard Android pull-to-refresh on the timeline list. Maps to `pulse sync run --feed current-group`.

## Performance Architecture (UI Layer)

### List Virtualization

The timeline list can contain thousands of items. Render only visible items using a virtual scroll implementation. The TanStack Virtual library (framework-agnostic) or a Svelte-native implementation handles this.

Only the visible viewport + 2 screens above/below are rendered. This keeps DOM size constant regardless of timeline length.

### Image Loading

Feed items may have thumbnail URLs (Reddit). Images are:
- Never loaded until visible (IntersectionObserver)
- Loaded with explicit `width` and `height` to prevent layout shift
- Cached by the browser via normal HTTP caching
- Optional: user can disable image loading for bandwidth savings

### Incremental Loading

The timeline loads in pages of 50 items. As the user scrolls toward the bottom, the next page is fetched. The loading state is surfaced as a subtle spinner at the bottom of the list, not a full-screen overlay.

## Keyboard Navigation (UI)

The power-user philosophy requires full keyboard navigation in the web UI as well:

| Key | Action |
|---|---|
| `j` / `↓` | Next item |
| `k` / `↑` | Previous item |
| `Enter` | Open reader |
| `Esc` | Close reader / go back |
| `o` | Open original URL |
| `r` | Toggle read |
| `s` | Toggle saved |
| `h` | Hide item |
| `/` | Focus search |
| `1-9` | Switch to group N |
| `?` | Show keyboard shortcuts |

Keyboard shortcuts are defined as a central map and bound via Svelte's `on:keydown` on the document root.

## Diagnostics Panel

The settings screen includes a diagnostics panel mirroring `pulse diag` output. This is particularly important on Android where users can't run the CLI. It shows:
- Database stats
- Feed health summary
- Sync engine state
- AI pipeline state
- Error log (last 50 errors)
