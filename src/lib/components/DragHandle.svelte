<script lang="ts">
  let { edge, onDragStart, dragging }: {
    edge: string;
    onDragStart: (edge: string, e: MouseEvent) => void;
    dragging: string | null;
  } = $props();
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  role="separator"
  onmousedown={(e) => onDragStart(edge, e)}
  style="width:4px;flex-shrink:0;cursor:col-resize;background:transparent;"
  onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--color-cyan-dim)'; }}
  onmouseleave={(e) => { if (dragging !== edge) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
></div>
