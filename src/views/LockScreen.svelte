<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { api, errorText, type Tree, type VaultStatus } from '../lib/api';
  import { BACKUP_FILTERS, describeBackup, fileName, restoreRememberedChoices } from '../lib/backup';
  import { toast } from '../lib/toast.svelte';

  let { status, onunlock }: { status: VaultStatus; onunlock: (tree: Tree) => void } = $props();

  /** First start, restoring a backup: the file it comes from. */
  let restoreFrom = $state<string | null>(null);
  const creating = $derived(!status.exists && !restoreFrom);
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

  async function chooseBackup() {
    try {
      const path = await open({
        title: 'Restore from a backup',
        multiple: false,
        directory: false,
        filters: [...BACKUP_FILTERS, { name: 'All files', extensions: ['*'] }],
      });
      if (typeof path !== 'string') return;
      restoreFrom = path;
      password = confirm = error = '';
    } catch (err) {
      error = errorText(err);
    }
  }

  function backToStart() {
    restoreFrom = null;
    password = confirm = error = '';
  }

  /** Nothing to replace yet, so no "Replace everything?" question. */
  async function restore(path: string): Promise<Tree> {
    const summary = await api.backupOpen(path, password);
    const restored = await api.backupRestore();
    restoreRememberedChoices(restored.ui);
    toast(`Restored ${describeBackup(summary)}.`);
    return restored.tree;
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    busy = true;
    error = '';
    try {
      const tree = restoreFrom
        ? await restore(restoreFrom)
        : creating
          ? await api.vaultCreate(password)
          : await api.vaultUnlock(password);
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
    {:else if restoreFrom}
      <h1 id="lock-title">Restore from a backup</h1>
      <p class="file" title={restoreFrom}>{fileName(restoreFrom)}</p>
      <input
        class="input"
        type="password"
        bind:value={password}
        autocomplete="current-password"
        placeholder="The master password it was made with"
        aria-label="Password for this backup"
        use:focus
      />
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
        {restoreFrom ? 'Restoring…' : creating ? 'Setting up…' : 'Unlocking…'}
      {:else}
        {restoreFrom ? 'Restore' : creating ? 'Get started' : 'Unlock'}
      {/if}
    </button>
    {#if creating}
      <button class="btn ghost" type="button" onclick={chooseBackup}>Restore from a backup…</button>
    {:else if restoreFrom}
      <button class="btn ghost" type="button" disabled={busy} onclick={backToStart}>Back</button>
    {/if}
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
  .file {
    overflow-wrap: anywhere;
  }
  .warn {
    color: var(--danger) !important;
  }
</style>
