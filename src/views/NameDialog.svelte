<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from './Modal.svelte';

  // Asks for a single name, e.g. for a new folder.
  let {
    title,
    label,
    initial = '',
    confirmLabel = 'Create',
    onsubmit,
    oncancel,
  }: {
    title: string;
    label: string;
    initial?: string;
    confirmLabel?: string;
    onsubmit: (name: string) => void;
    oncancel: () => void;
  } = $props();

  let name = $state(untrack(() => initial));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (name.trim()) onsubmit(name.trim());
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<Modal {title} {oncancel}>
  <form onsubmit={submit} class="form">
    <label class="field">
      <span>{label}</span>
      <input class="input" bind:value={name} use:focus />
    </label>
    <div class="actions">
      <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!name.trim()}>{confirmLabel}</button>
    </div>
  </form>
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
</style>
