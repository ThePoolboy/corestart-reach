<script lang="ts">
  import { untrack } from 'svelte';
  import { api, errorText, type Folder, type Protocol, type Saved, type Tree } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import { folderOptions } from '../lib/tree';

  let {
    tree,
    folder,
    ondirty,
    onsaved,
    ondelete,
    onnew,
    onnewfolder,
  }: {
    tree: Tree;
    folder: Folder;
    ondirty: (dirty: boolean) => void;
    onsaved: (saved: Saved) => void;
    ondelete: (folder: Folder) => void;
    onnew: (protocol: Protocol, folderId: string) => void;
    onnewfolder: (parentId: string) => void;
  } = $props();

  const start = untrack(() => ({ name: folder.name, parentId: folder.parentId ?? '' }));
  let form = $state({ ...start });
  let error = $state('');

  const dirty = $derived(form.name !== start.name || form.parentId !== start.parentId);
  $effect(() => ondirty(dirty));
  const parents = $derived(folderOptions(tree.folders, folder.id));
  const connections = $derived(tree.connections.filter((c) => c.folderId === folder.id).length);
  const subfolders = $derived(tree.folders.filter((f) => f.parentId === folder.id).length);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    try {
      onsaved(await api.saveFolder({ id: folder.id, name: form.name, parentId: form.parentId || null }));
    } catch (err) {
      error = errorText(err);
    }
  }
</script>

<form class="pane" onsubmit={save}>
  <header>
    <div class="icon"><Icon name="folder" size={22} /></div>
    <div>
      <h1>{folder.name}</h1>
      <div class="sub">
        {connections} connection{connections === 1 ? '' : 's'}{subfolders
          ? `, ${subfolders} folder${subfolders === 1 ? '' : 's'}`
          : ''}
      </div>
    </div>
  </header>

  <div class="body">
    <div class="quick">
      <button type="button" class="btn" onclick={() => onnew('rdp', folder.id)}>
        <Icon name="monitor" /> New RDP here
      </button>
      <button type="button" class="btn" onclick={() => onnew('ssh', folder.id)}>
        <Icon name="terminal" /> New SSH here
      </button>
      <button type="button" class="btn" onclick={() => onnewfolder(folder.id)}>
        <Icon name="folder" /> New subfolder
      </button>
    </div>

    <label class="field">
      <span>Name</span>
      <input class="input" bind:value={form.name} />
    </label>
    <label class="field">
      <span>Inside</span>
      <select class="input" bind:value={form.parentId}>
        <option value="">(Top level)</option>
        {#each parents as p (p.id)}
          <option value={p.id}>{p.label}</option>
        {/each}
      </select>
    </label>

    {#if error}
      <div class="error-text">{error}</div>
    {/if}

    <div class="buttons">
      <button type="button" class="btn danger" onclick={() => ondelete(folder)}>
        <Icon name="trash" /> Delete folder
      </button>
      <span class="spacer"></span>
      {#if dirty}
        <button type="button" class="btn" onclick={() => (form = { ...start })}>Revert</button>
        <button type="submit" class="btn primary" disabled={!form.name.trim()}>Save</button>
      {/if}
    </div>
  </div>
</form>

<style>
  .pane {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px 28px 16px;
    border-bottom: 1px solid var(--line);
  }
  .icon {
    color: var(--accent);
    display: grid;
  }
  h1 {
    margin: 0;
    font-size: 20px;
  }
  .sub {
    color: var(--text-dim);
    font-size: 12.5px;
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 640px;
    padding: 24px 28px;
  }
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding-bottom: 8px;
  }
  .buttons {
    display: flex;
    gap: 8px;
    padding-top: 8px;
  }
  .spacer {
    flex: 1;
  }
</style>
