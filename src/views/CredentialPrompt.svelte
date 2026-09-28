<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from './Modal.svelte';

  // Shown when an RDP connection has no saved username or password. What is
  // typed here is used for this launch only and never saved.
  let {
    name,
    username: initialUser,
    domain,
    onsubmit,
    oncancel,
  }: {
    name: string;
    username: string;
    domain: string;
    onsubmit: (username: string, password: string) => void;
    oncancel: () => void;
  } = $props();

  let username = $state(untrack(() => initialUser));
  let password = $state('');

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (username.trim()) onsubmit(username.trim(), password);
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

<Modal title="Log in to {name}" {oncancel}>
  <form onsubmit={submit} class="form">
    <label class="field">
      <span>Username{domain ? ` (domain ${domain})` : ''}</span>
      {#if initialUser}
        <input class="input" bind:value={username} />
      {:else}
        <input class="input" bind:value={username} use:focus />
      {/if}
    </label>
    <label class="field">
      <span>Password</span>
      {#if initialUser}
        <input class="input" type="password" bind:value={password} use:focus />
      {:else}
        <input class="input" type="password" bind:value={password} />
      {/if}
      <span class="hint">Used for this connection only. Edit the connection to save it.</span>
    </label>
    <div class="actions">
      <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!username.trim()}>Connect</button>
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
