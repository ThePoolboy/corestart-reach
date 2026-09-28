<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import { api, errorText, type Tree } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import { toast } from '../lib/toast.svelte';

  let { tree, onchange }: { tree: Tree; onchange: (t: Tree) => void } = $props();

  const AUTO_LOCK = [
    { minutes: 5, label: 'After 5 minutes' },
    { minutes: 10, label: 'After 10 minutes' },
    { minutes: 15, label: 'After 15 minutes' },
    { minutes: 30, label: 'After 30 minutes' },
    { minutes: 60, label: 'After 1 hour' },
    { minutes: 240, label: 'After 4 hours' },
    { minutes: 0, label: 'Never' },
  ];

  let version = $state('');
  onMount(async () => {
    version = await getVersion().catch(() => '');
  });

  async function setAutoLock(e: Event) {
    const minutes = Number((e.currentTarget as HTMLSelectElement).value);
    try {
      onchange(await api.saveSettings({ ...tree.settings, autoLockMinutes: minutes }));
      toast(minutes ? 'Auto-lock updated.' : 'Auto-lock turned off.');
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  // ---- change master password ----
  let current = $state('');
  let next = $state('');
  let confirm = $state('');
  let reveal = $state(false);
  let busy = $state(false);
  let error = $state('');

  const MIN = 8;
  const mismatch = $derived(confirm.length > 0 && confirm !== next);
  const canChange = $derived(!busy && current !== '' && next.length >= MIN && confirm === next);

  async function changePassword(e: SubmitEvent) {
    e.preventDefault();
    if (!canChange) return;
    busy = true;
    error = '';
    try {
      await api.vaultChangePassword(current, next);
      current = next = confirm = '';
      toast('Master password changed.');
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="pane">
  <header>
    <div class="icon"><Icon name="settings" size={22} /></div>
    <div>
      <h1>Settings</h1>
      <div class="sub">Stored inside your encrypted vault.</div>
    </div>
  </header>

  <div class="scroll">
    <section>
      <h2>Security</h2>
      <label class="field">
        <span>Lock the vault when idle</span>
        <select class="input" value={String(tree.settings.autoLockMinutes)} onchange={setAutoLock}>
          {#each AUTO_LOCK as o (o.minutes)}
            <option value={String(o.minutes)}>{o.label}</option>
          {/each}
          {#if !AUTO_LOCK.some((o) => o.minutes === tree.settings.autoLockMinutes)}
            <option value={String(tree.settings.autoLockMinutes)}>After {tree.settings.autoLockMinutes} minutes</option>
          {/if}
        </select>
        <span class="hint">
          Counts time you're not using the Reach window, including while the computer sleeps. Open RDP and SSH
          sessions keep running when it locks.
        </span>
      </label>
    </section>

    <section>
      <h2>Master password</h2>
      <form class="form" onsubmit={changePassword}>
        <label class="field">
          <span>Current password</span>
          <input class="input" type={reveal ? 'text' : 'password'} bind:value={current} autocomplete="current-password" />
        </label>
        <div class="row">
          <label class="field">
            <span>New password</span>
            <input class="input" type={reveal ? 'text' : 'password'} bind:value={next} autocomplete="new-password" />
            {#if next.length > 0 && next.length < MIN}
              <span class="hint">At least {MIN} characters.</span>
            {/if}
          </label>
          <label class="field">
            <span>Confirm new password</span>
            <input class="input" type={reveal ? 'text' : 'password'} bind:value={confirm} autocomplete="new-password" />
            {#if mismatch}
              <span class="hint warn">Passwords don't match.</span>
            {/if}
          </label>
        </div>
        {#if error}
          <div class="error-text">{error}</div>
        {/if}
        <div class="buttons">
          <button type="button" class="btn ghost" onclick={() => (reveal = !reveal)}>
            {reveal ? 'Hide passwords' : 'Show passwords'}
          </button>
          <span class="spacer"></span>
          <button type="submit" class="btn primary" disabled={!canChange}>
            {busy ? 'Changing…' : 'Change password'}
          </button>
        </div>
        <p class="hint">
          Re-encrypts the vault and its backup with the new password. There is still no way to recover it if you
          forget it.
        </p>
      </form>
    </section>

    <section>
      <h2>About</h2>
      <p class="about">
        Corestart Reach{version ? ` ${version}` : ''} · Free software under the GPL-3.0 license.<br />
        Source code: github.com/ThePoolboy/corestart-reach
      </p>
    </section>
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
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 28px 28px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 14px;
    max-width: 640px;
    padding: 18px 0;
    border-bottom: 1px solid var(--line);
  }
  section:last-child {
    border-bottom: 0;
  }
  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .buttons {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spacer {
    flex: 1;
  }
  p.hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-faint);
  }
  .warn {
    color: var(--danger) !important;
  }
  .about {
    margin: 0;
    color: var(--text-dim);
    user-select: text;
    -webkit-user-select: text;
  }
</style>
