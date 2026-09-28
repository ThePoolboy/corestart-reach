<script lang="ts">
  import { api, errorText, type Tree, type VaultStatus } from '../lib/api';
  import Icon from '../lib/Icon.svelte';

  let { status, onunlock }: { status: VaultStatus; onunlock: (tree: Tree) => void } = $props();

  const creating = $derived(!status.exists);
  let password = $state('');
  let confirm = $state('');
  let busy = $state(false);
  let error = $state('');

  const tooShort = $derived(creating && password.length > 0 && password.length < status.minPasswordLength);
  const mismatch = $derived(creating && confirm.length > 0 && confirm !== password);
  const canSubmit = $derived(
    !busy &&
      password.length > 0 &&
      (!creating || (password.length >= status.minPasswordLength && confirm === password)),
  );

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    busy = true;
    error = '';
    try {
      const tree = creating ? await api.vaultCreate(password) : await api.vaultUnlock(password);
      password = '';
      confirm = '';
      onunlock(tree);
    } catch (err) {
      error = errorText(err);
      password = '';
      confirm = '';
    } finally {
      busy = false;
    }
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

<main class="lock">
  <form class="card" onsubmit={submit}>
    <img class="logo" src="/icon.svg" alt="" width="60" height="60" />
    <h1>Corestart Reach</h1>

    {#if creating}
      <p>
        Choose a master password. It encrypts every host, username and password you save in Reach.
      </p>
      <label class="field">
        <span>Master password</span>
        <input class="input" type="password" bind:value={password} autocomplete="new-password" use:focus />
        {#if tooShort}
          <span class="hint">At least {status.minPasswordLength} characters.</span>
        {/if}
      </label>
      <label class="field">
        <span>Confirm password</span>
        <input class="input" type="password" bind:value={confirm} autocomplete="new-password" />
        {#if mismatch}
          <span class="hint warn">Passwords don't match.</span>
        {/if}
      </label>
      <div class="note">
        <Icon name="shield" />
        <span>There is no way to recover this password. If you forget it, your saved connections are gone.</span>
      </div>
    {:else}
      <p>Enter your master password to unlock your connections.</p>
      <label class="field">
        <span>Master password</span>
        <input class="input" type="password" bind:value={password} autocomplete="current-password" use:focus />
      </label>
    {/if}

    {#if error}
      <div class="error-text">{error}</div>
    {/if}

    <button class="btn primary large" type="submit" disabled={!canSubmit}>
      <Icon name="lock" />
      {#if busy}
        {creating ? 'Creating…' : 'Unlocking…'}
      {:else}
        {creating ? 'Create vault' : 'Unlock'}
      {/if}
    </button>

    <div class="path" title={status.path}>{status.path}</div>
  </form>
</main>

<style>
  /* Scrolls instead of clipping when the window is short. The card centres
     with margin:auto, which (unlike place-items) never cuts off its top. */
  .lock {
    height: 100%;
    overflow-y: auto;
    display: flex;
    padding: 24px;
    background:
      radial-gradient(circle at 50% 0%, rgba(35, 160, 243, 0.12), transparent 60%),
      var(--bg);
  }
  .card {
    width: min(400px, 100%);
    margin: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 24px 28px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--bg-raised);
    box-shadow: var(--shadow);
  }
  .logo {
    align-self: center;
  }
  h1 {
    margin: 0;
    text-align: center;
    font-size: 21px;
  }
  p {
    margin: 0;
    color: var(--text-dim);
    text-align: center;
  }
  .note {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-hover);
    color: var(--text-dim);
    font-size: 12.5px;
  }
  .note :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--accent);
  }
  .warn {
    color: var(--danger) !important;
  }
  .path {
    font-size: 11px;
    color: var(--text-faint);
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
