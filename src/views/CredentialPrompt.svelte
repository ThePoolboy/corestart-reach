<script lang="ts">
  import { untrack } from 'svelte';
  import type { Login } from '../lib/api';
  import Modal from './Modal.svelte';

  // Shown when an RDP connection has no saved username or password. What is
  // typed here is used for this launch only and never saved.
  let {
    name,
    username: initialUser,
    domain: initialDomain,
    onsubmit,
    oncancel,
  }: {
    name: string;
    username: string;
    domain: string;
    onsubmit: (login: Login) => void;
    oncancel: () => void;
  } = $props();

  let username = $state(untrack(() => initialUser));
  let domain = $state(untrack(() => initialDomain));
  let password = $state('');
  let reveal = $state(false);

  const canSubmit = $derived(username.trim() !== '' && password !== '');
  // "CORP\admin" or "admin@corp.example.com" already carry the domain.
  const domainInUser = $derived(/[\\@]/.test(username));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (canSubmit) onsubmit({ username: username.trim(), domain: domainInUser ? '' : domain.trim(), password });
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

<Modal title="Log in to {name}" {oncancel}>
  <form onsubmit={submit} class="form">
    <div class="row">
      <label class="field">
        <span>Username</span>
        {#if initialUser}
          <input class="input" bind:value={username} spellcheck="false" autocomplete="off" />
        {:else}
          <input class="input" bind:value={username} spellcheck="false" autocomplete="off" use:focus />
        {/if}
      </label>
      <label class="field">
        <span>Domain</span>
        <input
          class="input"
          bind:value={domain}
          placeholder={domainInUser ? 'in username' : 'optional'}
          disabled={domainInUser}
          spellcheck="false"
        />
      </label>
    </div>
    <div class="field">
      <label for="login-password">Password</label>
      <div class="secret">
        {#if initialUser}
          <input id="login-password" class="input" type={reveal ? 'text' : 'password'} bind:value={password} use:focus />
        {:else}
          <input id="login-password" class="input" type={reveal ? 'text' : 'password'} bind:value={password} />
        {/if}
        <button type="button" class="btn ghost" onclick={() => (reveal = !reveal)} aria-pressed={reveal}>
          {reveal ? 'Hide' : 'Show'}
        </button>
      </div>
    </div>
    <p class="hint">
      Domain accounts: fill in Domain, or type <code>CORP\user</code> or <code>user@corp.example.com</code>.
      Used for this connection only, never saved.
    </p>
    <div class="actions">
      <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!canSubmit}>Connect</button>
    </div>
  </form>
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .field > label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-dim);
  }
  .secret {
    display: flex;
    gap: 6px;
  }
  .secret .input {
    flex: 1;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-faint) !important;
  }
  code {
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--text-dim);
  }
</style>
