<script lang="ts">
  import { untrack } from 'svelte';
  import { api, errorText, type Credential, type Saved, type SecretUpdate } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import SecretField from './SecretField.svelte';

  let {
    credential,
    usedBy,
    ondirty,
    onsaved,
    ondelete,
    oncancel,
  }: {
    /** null when creating */
    credential: Credential | null;
    usedBy: number;
    ondirty: (dirty: boolean) => void;
    onsaved: (saved: Saved) => void;
    ondelete: (credential: Credential) => void;
    oncancel: () => void;
  } = $props();

  const start = untrack(() => ({
    name: credential?.name ?? '',
    username: credential?.username ?? '',
    domain: credential?.domain ?? '',
  }));
  const isNew = untrack(() => credential === null);

  let form = $state({ ...start });
  let password = $state<SecretUpdate>('keep');
  let error = $state('');

  const dirty = $derived(JSON.stringify(form) !== JSON.stringify(start) || password !== 'keep');

  $effect(() => ondirty(dirty));

  /** Bumped by Revert to reset the password field. */
  let resets = $state(0);

  async function save(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    try {
      onsaved(await api.saveCredential({ id: credential?.id ?? null, ...form, password }));
    } catch (err) {
      error = errorText(err);
    }
  }

  function focus(node: HTMLInputElement) {
    if (isNew) node.focus();
  }
</script>

<form class="form" onsubmit={save}>
  <h2>{isNew ? 'New credential' : credential?.name}</h2>
  <label class="field">
    <span>Name</span>
    <input class="input" bind:value={form.name} placeholder="e.g. Domain admin" use:focus />
  </label>
  <div class="row">
    <label class="field">
      <span>Username</span>
      <input class="input" bind:value={form.username} spellcheck="false" autocomplete="off" />
    </label>
    <label class="field">
      <span>Domain</span>
      <input class="input" bind:value={form.domain} placeholder="optional, RDP only" spellcheck="false" />
    </label>
  </div>
  {#key resets}
    <SecretField label="Password" saved={credential?.hasPassword ?? false} bind:value={password} />
  {/key}

  {#if error}
    <div class="error-text">{error}</div>
  {/if}

  <div class="buttons">
    {#if credential}
      <button type="button" class="btn danger" onclick={() => ondelete(credential)}>
        <Icon name="trash" /> Delete
      </button>
      <span class="used">Used by {usedBy} connection{usedBy === 1 ? '' : 's'}</span>
    {/if}
    <span class="spacer"></span>
    {#if isNew || dirty}
      {#if isNew}
        <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      {:else}
        <button
          type="button"
          class="btn"
          onclick={() => {
            form = { ...start };
            password = 'keep';
            resets++;
          }}>Revert</button
        >
      {/if}
      <button type="submit" class="btn primary">Save</button>
    {/if}
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 560px;
  }
  h2 {
    margin: 0;
    font-size: 16px;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 6px;
  }
  .used {
    color: var(--text-faint);
    font-size: 12.5px;
  }
  .spacer {
    flex: 1;
  }
</style>
