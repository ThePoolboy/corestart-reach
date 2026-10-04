<script lang="ts">
  import { api, errorText, type Tree, type VaultStatus } from '../lib/api';

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

<!-- The logo and the login on a card in the middle of the window. No app name:
     the window title has it. Where the data file lives is in Settings → About. -->
<main class="lock">
  <form class="card" onsubmit={submit} aria-labelledby="lock-title">
    <img class="logo" src="/icon.svg" alt="" width="60" height="60" />

    {#if creating}
      <h1 id="lock-title">Set a master password</h1>
      <p>It encrypts everything you save in Reach.</p>
      <label class="field">
        <span>Master password</span>
        <input
          class="input"
          type="password"
          bind:value={password}
          autocomplete="new-password"
          placeholder="At least {status.minPasswordLength} characters"
          use:focus
        />
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
      <p class="hint">There's no way to recover this password. If you forget it, your saved connections are gone.</p>
    {:else}
      <h1 id="lock-title">Unlock your connections</h1>
      <input
        class="input"
        type="password"
        bind:value={password}
        autocomplete="current-password"
        placeholder="Master password"
        aria-label="Master password"
        use:focus
      />
    {/if}

    {#if error}
      <div class="error-text">{error}</div>
    {/if}

    <button class="btn primary large" type="submit" disabled={!canSubmit}>
      {#if busy}
        {creating ? 'Setting up…' : 'Unlocking…'}
      {:else}
        {creating ? 'Get started' : 'Unlock'}
      {/if}
    </button>
  </form>
</main>

<style>
  /* Scrolls instead of clipping when the window is short. The column centres
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
    margin-bottom: 4px;
  }
  h1 {
    margin: 0 0 4px;
    text-align: center;
    font-size: 21px;
  }
  p {
    margin: 0;
    color: var(--text-dim);
    text-align: center;
    text-wrap: balance;
  }
  p.hint {
    font-size: 12.5px;
    text-align: left;
    text-wrap: wrap;
  }
  .card .input,
  .card .btn {
    height: 38px;
  }
  .error-text {
    text-align: center;
  }
  .warn {
    color: var(--danger) !important;
  }
</style>
