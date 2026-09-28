<script lang="ts">
  import type { Snippet } from 'svelte';

  // A centred dialog. Escape or a click on the backdrop cancels it.
  let {
    title,
    oncancel,
    children,
  }: { title: string; oncancel: () => void; children: Snippet } = $props();

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      oncancel();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={(e) => e.target === e.currentTarget && oncancel()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
    <h2>{title}</h2>
    {@render children()}
  </div>
</div>
