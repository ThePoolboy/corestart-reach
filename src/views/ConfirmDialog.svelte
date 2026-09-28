<script lang="ts">
  import Modal from './Modal.svelte';

  let {
    title,
    message,
    confirmLabel = 'Delete',
    onconfirm,
    oncancel,
  }: {
    title: string;
    message: string;
    confirmLabel?: string;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  function focus(node: HTMLButtonElement) {
    node.focus();
  }
</script>

<Modal {title} {oncancel}>
  <p>{message}</p>
  <div class="actions">
    <button class="btn" onclick={oncancel}>Cancel</button>
    <button class="btn danger" onclick={onconfirm} use:focus>{confirmLabel}</button>
  </div>
</Modal>
