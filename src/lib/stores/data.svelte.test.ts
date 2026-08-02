import { describe, it, expect, beforeEach } from 'vitest';
import { items, toggleSaved, setNote, saveWithNote } from './data.svelte';

// Regression tests for the "save with note" bug (R1):
// a note-save must never unsave an item, and notes must be clearable.

function fresh() {
  const item = items[0];
  item.saved = false;
  item.note = undefined;
  return item;
}

describe('save/note semantics (non-Tauri optimistic path)', () => {
  beforeEach(fresh);

  it('saveWithNote keeps an already-saved item saved and sets the note', async () => {
    const item = items[0];
    item.saved = true;
    await saveWithNote(item.id, 'my note');
    expect(item.saved).toBe(true);
    expect(item.note).toBe('my note');
  });

  it('saveWithNote saves an unsaved item and sets the note', async () => {
    const item = items[0];
    item.saved = false;
    await saveWithNote(item.id, 'my note');
    expect(item.saved).toBe(true);
    expect(item.note).toBe('my note');
  });

  it('saveWithNote with blank text clears the note but keeps the item saved', async () => {
    const item = items[0];
    item.saved = true;
    item.note = 'old note';
    await saveWithNote(item.id, '   ');
    expect(item.saved).toBe(true);
    expect(item.note).toBeUndefined();
  });

  it('toggleSaved flips the saved flag and never touches the note', async () => {
    const item = items[0];
    item.saved = false;
    item.note = 'keep me';
    await toggleSaved(item.id);
    expect(item.saved).toBe(true);
    expect(item.note).toBe('keep me');
    await toggleSaved(item.id);
    expect(item.saved).toBe(false);
    expect(item.note).toBe('keep me');
  });

  it('setNote does not touch the saved flag', async () => {
    const item = items[0];
    item.saved = false;
    await setNote(item.id, 'note');
    expect(item.saved).toBe(false);
    expect(item.note).toBe('note');
  });

  it('setNote(null) clears the note', async () => {
    const item = items[0];
    item.note = 'old';
    await setNote(item.id, null);
    expect(item.note).toBeUndefined();
  });
});
