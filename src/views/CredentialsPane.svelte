<script lang="ts">
  import type { Credential, Saved, Tree } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import { byName } from '../lib/tree';
  import CredentialEditor from './CredentialEditor.svelte';

  // Saved credentials: one username/password shared by many connections.
  let {
    tree,
    selected,
    ondirty,
    onselect,
    onsaved,
    ondelete,
  }: {
    tree: Tree;
    /** id of the credential being edited, 'new', or null */
    selected: string | null;
    ondirty: (dirty: boolean) => void;
    onselect: (id: string | null) => void;
    onsaved: (saved: Saved) => void;
    ondelete: (credential: Credential, usedBy: number) => void;
  } = $props();

  const list = $derived([...tree.credentials].sort(byName));
  const current = $derived(tree.credentials.find((c) => c.id === selected) ?? null);
  const usage = (id: string) => tree.connections.filter((c) => c.credentialId === id).length;
</script>

<div class="pane">
  <header>
    <div class="icon"><Icon name="key" size={22} /></div>
    <div>
      <h1>Credentials</h1>
      <div class="sub">Save a login once and use it on many connections.</div>
    </div>
    <span class="spacer"></span>
    <button class="btn primary" onclick={() => onselect('new')}><Icon name="plus" /> New credential</button>
  </header>

  <div class="split">
    <ul class="list">
      {#each list as c (c.id)}
        <li>
          <button class:on={selected === c.id} onclick={() => onselect(c.id)}>
            <strong>{c.name}</strong>
            <span>{c.domain ? `${c.domain}\\` : ''}{c.username || 'no username'} · used by {usage(c.id)}</span>
          </button>
        </li>
      {:else}
        <li class="empty">No saved credentials yet.</li>
      {/each}
    </ul>

    <div class="detail">
      {#if selected === 'new' || current}
        {#key selected}
          <CredentialEditor
            credential={current}
            usedBy={current ? usage(current.id) : 0}
            {ondirty}
            {onsaved}
            ondelete={(c) => ondelete(c, usage(c.id))}
            oncancel={() => onselect(null)}
          />
        {/key}
      {:else}
        <p class="hint-text">Select a credential to edit it, or create a new one.</p>
      {/if}
    </div>
  </div>
</div>

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
  .spacer {
    flex: 1;
  }
  .split {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(200px, 280px) 1fr;
    min-height: 0;
  }
  .list {
    margin: 0;
    padding: 8px;
    list-style: none;
    overflow-y: auto;
    border-right: 1px solid var(--line);
  }
  .list button {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .list button:hover {
    background: var(--bg-hover);
  }
  .list button.on {
    background: var(--bg-selected);
  }
  .list span {
    font-size: 12px;
    color: var(--text-dim);
  }
  .empty,
  .hint-text {
    padding: 12px;
    color: var(--text-faint);
  }
  .detail {
    overflow-y: auto;
    padding: 20px 28px;
  }
</style>
