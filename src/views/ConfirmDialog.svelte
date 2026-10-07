<script lang="ts">
  import Modal from './Modal.svelte';

  let {
    title,
    message,
    confirmLabel = 'Delete',
    cancelFirst = false,
    onconfirm,
    oncancel,
  }: {
    title: string;
    message: string;
    confirmLabel?: string;
    /** Start on Cancel, so Enter doesn't confirm something hard to undo. */
    cancelFirst?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();

  function focus(node: HTMLButtonElement, on: boolean) {
    if (on) node.focus();
  }
</script>

<Modal {title} {oncancel}>
  <p>{message}</p>
  <div class="actions">
    <button class="btn" onclick={oncancel} use:focus={cancelFirst}>Cancel</button>
    <button class="btn danger" onclick={onconfirm} use:focus={!cancelFirst}>{confirmLabel}</button>
  </div>
</Modal>
