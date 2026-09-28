<script lang="ts">
  import { untrack } from 'svelte';
  import type { Folder } from '../lib/api';
  import { folderOptions } from '../lib/tree';
  import Modal from './Modal.svelte';

  // "Move to…": pick a destination folder for a connection or folder.
  let {
    name,
    folders,
    current,
    exclude,
    onsubmit,
    oncancel,
  }: {
    name: string;
    folders: Folder[];
    /** where the item is now (null = top level) */
    current: string | null;
    /** when moving a folder: it and its subfolders can't be the destination */
    exclude?: string;
    onsubmit: (folderId: string | null) => void;
    oncancel: () => void;
  } = $props();

  const options = $derived(folderOptions(folders, exclude));
  let target = $state(untrack(() => current ?? ''));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (target !== (current ?? '')) onsubmit(target || null);
  }

  function focus(node: HTMLSelectElement) {
    node.focus();
  }
</script>

<Modal title="Move {name}" {oncancel}>
  <form onsubmit={submit} class="form">
    <label class="field">
      <span>Move to</span>
      <select class="input" bind:value={target} use:focus>
        <option value="">(Top level)</option>
        {#each options as f (f.id)}
          <option value={f.id}>{f.label}</option>
        {/each}
      </select>
      <span class="hint">You can also drag items onto a folder in the sidebar.</span>
    </label>
    <div class="actions">
      <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      <button type="submit" class="btn primary" disabled={target === (current ?? '')}>Move</button>
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
