<script lang="ts">
  import { getVersion } from '@tauri-apps/api/app';
  import { documentDir, join } from '@tauri-apps/api/path';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { onMount } from 'svelte';
  import bitcoinQr from '../../assets/donate/bitcoin.png';
  import ethereumQr from '../../assets/donate/ethereum.png';
  import { api, errorText, type BackupSummary, type Tree } from '../lib/api';
  import {
    BACKUP_FILTERS,
    backupDate,
    backupFileName,
    describeBackup,
    fileName,
    rememberedChoices,
    restoreRememberedChoices,
  } from '../lib/backup';
  import Icon from '../lib/Icon.svelte';
  import { SHORTCUTS, defaultKeys, keysFor, keysOf, problemWith, type ShortcutAction } from '../lib/shortcuts';
  import { setTheme, theme, type ThemeChoice } from '../lib/theme.svelte';
  import { toast } from '../lib/toast.svelte';
  import { checkForUpdate, updates } from '../lib/update.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';

  let {
    tree,
    onchange,
    onrestore,
  }: { tree: Tree; onchange: (t: Tree) => void; onrestore: (t: Tree) => void } = $props();

  const AUTO_LOCK = [
    { minutes: 5, label: 'After 5 minutes' },
    { minutes: 10, label: 'After 10 minutes' },
    { minutes: 15, label: 'After 15 minutes' },
    { minutes: 30, label: 'After 30 minutes' },
    { minutes: 60, label: 'After 1 hour' },
    { minutes: 240, label: 'After 4 hours' },
    { minutes: 0, label: 'Never' },
  ];

  const THEMES: { choice: ThemeChoice; label: string }[] = [
    { choice: 'system', label: 'System' },
    { choice: 'light', label: 'Light' },
    { choice: 'dark', label: 'Dark' },
  ];

  let version = $state('');
  /** Where the data file is, for backups. */
  let vaultPath = $state('');
  onMount(async () => {
    version = await getVersion().catch(() => '');
    vaultPath = (await api.vaultStatus().catch(() => null))?.path ?? '';
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

  // ---- keyboard shortcuts ----
  /** The shortcut waiting for its new keys, if any. */
  let recording = $state<ShortcutAction | null>(null);
  let keysError = $state('');

  function startRecording(action: ShortcutAction) {
    keysError = '';
    recording = recording === action ? null : action;
  }

  function stopRecording() {
    recording = null;
    keysError = '';
  }

  // While recording, the next key press is the new shortcut rather than a
  // command, so it's caught before the window's own shortcuts see it.
  $effect(() => {
    const action = recording;
    if (!action) return;
    const onKey = (e: KeyboardEvent) => {
      const plain = !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey;
      if (plain && e.key === 'Tab') return stopRecording(); // Tab still moves on
      e.preventDefault();
      e.stopImmediatePropagation();
      if (plain && e.key === 'Escape') return stopRecording();
      const keys = keysOf(e);
      if (!keys) return; // only modifiers so far
      const problem = problemWith(e, keys, action, tree.settings);
      if (problem) {
        keysError = problem;
        return;
      }
      stopRecording();
      saveShortcut(action, keys);
    };
    window.addEventListener('keydown', onKey, { capture: true });
    return () => window.removeEventListener('keydown', onKey, { capture: true });
  });

  /** Keep only the shortcuts that differ from their defaults. */
  async function saveShortcut(action: ShortcutAction, keys: string) {
    const shortcuts = { ...tree.settings.shortcuts };
    if (keys === defaultKeys(action)) delete shortcuts[action];
    else shortcuts[action] = keys;
    try {
      onchange(await api.saveSettings({ ...tree.settings, shortcuts }));
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  // ---- backup ----
  /** The export or import waiting for a password, and its file. */
  let backup = $state<{ kind: 'export' | 'import'; path: string } | null>(null);
  let backupPassword = $state('');
  let backupBusy = $state(false);
  let backupError = $state('');
  /** An opened backup, waiting for "Replace everything". */
  let opened = $state<BackupSummary | null>(null);
  const replaceMessage = $derived(
    opened &&
      `The backup from ${backupDate(opened)} has ${describeBackup(opened)}. It replaces all your connections ` +
        "and settings here, and Reach will unlock with the backup's master password from now on. " +
        'What you have now is kept as a copy next to the data file.',
  );

  async function startExport() {
    try {
      const name = backupFileName();
      const start = await documentDir()
        .then((dir) => join(dir, name))
        .catch(() => name);
      const path = await save({ title: 'Export a backup', defaultPath: start, filters: BACKUP_FILTERS });
      if (path) showBackupForm('export', path);
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  async function startImport() {
    try {
      const path = await open({
        title: 'Import a backup',
        multiple: false,
        directory: false,
        filters: [...BACKUP_FILTERS, { name: 'All files', extensions: ['*'] }],
      });
      if (typeof path === 'string') showBackupForm('import', path);
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  function showBackupForm(kind: 'export' | 'import', path: string) {
    cancelBackup();
    backup = { kind, path };
  }

  function cancelBackup() {
    backup = null;
    backupPassword = '';
    backupError = '';
  }

  async function submitBackup(e: SubmitEvent) {
    e.preventDefault();
    if (!backup || !backupPassword || backupBusy) return;
    const { kind, path } = backup;
    backupBusy = true;
    backupError = '';
    try {
      if (kind === 'export') {
        await api.backupExport(path, backupPassword, rememberedChoices());
        toast(`Backup saved as ${fileName(path)}.`);
      } else {
        opened = await api.backupOpen(path, backupPassword);
      }
      cancelBackup();
    } catch (err) {
      backupError = errorText(err);
      backupPassword = '';
    } finally {
      backupBusy = false;
    }
  }

  async function replaceEverything() {
    opened = null;
    try {
      const restored = await api.backupRestore();
      restoreRememberedChoices(restored.ui);
      onrestore(restored.tree);
      toast("Backup imported. Reach now unlocks with the backup's master password.");
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  function keepCurrent() {
    opened = null;
    api.backupCancel().catch(() => {});
  }

  // ---- updates ----
  async function setCheckForUpdates(on: boolean) {
    try {
      onchange(await api.saveSettings({ ...tree.settings, checkForUpdates: on }));
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  async function checkNow() {
    try {
      const found = await checkForUpdate();
      toast(found ? `Corestart Reach ${found.version} is available.` : "You're on the latest version.");
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  // ---- about and support ----
  const WALLETS = [
    { name: 'Bitcoin (BTC)', address: 'bc1qrwfpl77k7zfgrea8suv78cxn8wp3lvn3mjumz0', qr: bitcoinQr },
    { name: 'Ethereum (ETH)', address: '0xB968531aa4f6EaE2c2c479B56b111c8B3B5c6C54', qr: ethereumQr },
  ];
  let showQr = $state(false);
  let showCrypto = $state(false);

  async function openLink(link: 'source' | 'sponsors' | 'kofi') {
    try {
      await api.openLink(link);
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  async function copyAddress(name: string, address: string) {
    try {
      await writeText(address);
      toast(`${name} address copied.`);
    } catch (err) {
      toast(errorText(err), 'error');
    }
  }

  // ---- change master password ----
  let current = $state('');
  let next = $state('');
  let confirm = $state('');
  let reveal = $state(false);
  let changing = $state(false);
  let busy = $state(false);
  let error = $state('');

  const MIN = 8;
  const mismatch = $derived(confirm.length > 0 && confirm !== next);
  const canChange = $derived(!busy && current !== '' && next.length >= MIN && confirm === next);

  function cancelChange() {
    changing = false;
    reveal = false;
    error = '';
    current = next = confirm = '';
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }

  async function changePassword(e: SubmitEvent) {
    e.preventDefault();
    if (!canChange) return;
    busy = true;
    error = '';
    try {
      await api.vaultChangePassword(current, next);
      cancelChange();
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
    <h1>Settings</h1>
  </header>

  <div class="scroll">
    <section>
      <h2>Appearance</h2>
      <div class="field">
        <span id="theme-label">Theme</span>
        <div class="segmented" role="radiogroup" aria-labelledby="theme-label">
          {#each THEMES as t (t.choice)}
            <button
              type="button"
              role="radio"
              aria-checked={theme.choice === t.choice}
              class:on={theme.choice === t.choice}
              onclick={() => setTheme(t.choice)}>{t.label}</button
            >
          {/each}
        </div>
      </div>
    </section>

    <section>
      <h2>Security</h2>
      <label class="field">
        <span>Lock Reach when idle</span>
        <select class="input" value={String(tree.settings.autoLockMinutes)} onchange={setAutoLock}>
          {#each AUTO_LOCK as o (o.minutes)}
            <option value={String(o.minutes)}>{o.label}</option>
          {/each}
          {#if !AUTO_LOCK.some((o) => o.minutes === tree.settings.autoLockMinutes)}
            <option value={String(tree.settings.autoLockMinutes)}>After {tree.settings.autoLockMinutes} minutes</option>
          {/if}
        </select>
        <span class="hint">Open RDP and SSH sessions keep running after Reach locks.</span>
      </label>
      {#if !changing}
        <div class="buttons">
          <button type="button" class="btn" onclick={() => (changing = true)}>Change master password…</button>
        </div>
      {:else}
        <form class="form" onsubmit={changePassword}>
          <label class="field">
            <span>Current password</span>
            <input
              class="input"
              type={reveal ? 'text' : 'password'}
              bind:value={current}
              autocomplete="current-password"
              use:focus
            />
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
            <button type="button" class="btn" onclick={cancelChange}>Cancel</button>
            <button type="submit" class="btn primary" disabled={!canChange}>
              {busy ? 'Changing…' : 'Change password'}
            </button>
          </div>
          <p class="hint">
            Re-encrypts your saved data and its backup with the new password. There is still no way to recover it if
            you forget it.
          </p>
        </form>
      {/if}
    </section>

    <section>
      <h2>Backup</h2>
      {#if !backup}
        <div class="buttons">
          <button type="button" class="btn" title="Save everything in Reach to one encrypted file" onclick={startExport}>
            Export backup…
          </button>
          <button type="button" class="btn" title="Replace everything in Reach with a backup" onclick={startImport}>
            Import backup…
          </button>
        </div>
      {:else}
        {@const exporting = backup.kind === 'export'}
        <form class="form" onsubmit={submitBackup}>
          <p class="about">
            {exporting ? 'Export to' : 'Import from'} <span class="path" title={backup.path}>{fileName(backup.path)}</span>
          </p>
          <label class="field">
            <span>{exporting ? 'Master password' : 'Password for this backup'}</span>
            <input
              class="input"
              type="password"
              bind:value={backupPassword}
              autocomplete="current-password"
              placeholder={exporting ? '' : 'The master password it was made with'}
              use:focus
            />
          </label>
          {#if backupError}
            <div class="error-text">{backupError}</div>
          {/if}
          <div class="buttons">
            <span class="spacer"></span>
            <button type="button" class="btn" onclick={cancelBackup}>Cancel</button>
            <button type="submit" class="btn primary" disabled={!backupPassword || backupBusy}>
              {#if backupBusy}
                {exporting ? 'Exporting…' : 'Opening…'}
              {:else}
                {exporting ? 'Export' : 'Open'}
              {/if}
            </button>
          </div>
        </form>
      {/if}
    </section>

    <section>
      <h2>Updates</h2>
      {#if updates.supported}
        <div class="field">
          <span id="updates-label">Check for updates</span>
          <div class="buttons">
            <div class="segmented" role="radiogroup" aria-labelledby="updates-label">
              {#each [true, false] as on (on)}
                <button
                  type="button"
                  role="radio"
                  aria-checked={tree.settings.checkForUpdates === on}
                  class:on={tree.settings.checkForUpdates === on}
                  onclick={() => setCheckForUpdates(on)}>{on ? 'Automatically' : 'Only when I ask'}</button
                >
              {/each}
            </div>
            <button type="button" class="btn" disabled={updates.checking} onclick={checkNow}>
              {updates.checking ? 'Checking…' : 'Check now'}
            </button>
          </div>
          <span class="hint">Reach asks GitHub after you unlock it and every 12 hours. Nothing about you is sent.</span>
        </div>
      {:else}
        <p class="about">Your software center (or <code>flatpak update</code>) installs updates for Reach.</p>
      {/if}
    </section>

    <section>
      <h2>Keyboard shortcuts</h2>
      <div class="shortcuts">
        {#each SHORTCUTS as s (s.action)}
          {@const keys = keysFor(tree.settings, s.action)}
          <span>{s.label}</span>
          <button
            type="button"
            class="keys"
            class:recording={recording === s.action}
            aria-label="{s.label}: {keys}. Change"
            onclick={() => startRecording(s.action)}
            onblur={() => recording === s.action && stopRecording()}
          >
            {recording === s.action ? 'Press keys…' : keys}
          </button>
          <span>
            {#if keys !== s.keys}
              <button type="button" class="btn ghost" title="Back to {s.keys}" onclick={() => saveShortcut(s.action, s.keys)}>
                Reset
              </button>
            {/if}
          </span>
        {/each}
        <span>Copy / paste in SSH windows</span>
        <span class="keys fixed">Ctrl+Shift+C / V</span>
        <span></span>
      </div>
      <p class="hint" class:warn={keysError !== ''} aria-live="polite">
        {keysError || (recording ? 'Press the new keys, or Escape to cancel.' : 'Click a shortcut to change it.')}
      </p>
    </section>

    <section>
      <h2>About</h2>
      <p class="about">
        Corestart Reach{version ? ` ${version}` : ''} · Free software under the GPL-3.0 license.
      </p>
      {#if vaultPath}
        <p class="hint">Data file: <span class="path">{vaultPath}</span></p>
      {/if}
      <div class="buttons">
        <button type="button" class="btn" onclick={() => openLink('source')}>Source code on GitHub</button>
      </div>
    </section>

    <section>
      <h2>Support Reach</h2>
      <p class="about">Reach is free and always will be. Donations are optional and don't unlock anything.</p>
      <div class="buttons">
        <button type="button" class="btn" onclick={() => openLink('sponsors')}>Sponsor on GitHub</button>
        <button type="button" class="btn" onclick={() => openLink('kofi')}>Support on Ko-fi</button>
        <button type="button" class="btn ghost" aria-expanded={showCrypto} onclick={() => (showCrypto = !showCrypto)}>
          {showCrypto ? 'Hide crypto addresses' : 'Donate with crypto'}
        </button>
      </div>
      {#if showCrypto}
        <div class="wallets">
          {#each WALLETS as w (w.name)}
            <div class="wallet">
              <span class="wallet-name">{w.name}</span>
              <code class="address">{w.address}</code>
              <button
                type="button"
                class="btn icon"
                title="Copy {w.name} address"
                aria-label="Copy {w.name} address"
                onclick={() => copyAddress(w.name, w.address)}
              >
                <Icon name="copy" size={16} />
              </button>
              {#if showQr}
                <img class="qr" src={w.qr} alt="{w.name} QR code" width="140" height="140" />
              {/if}
            </div>
          {/each}
        </div>
        <div class="buttons">
          <button type="button" class="btn ghost" onclick={() => (showQr = !showQr)}>
            {showQr ? 'Hide QR codes' : 'Show QR codes'}
          </button>
        </div>
        <p class="hint">Check the address in your wallet before sending. Crypto payments can't be reversed.</p>
      {/if}
    </section>
  </div>
</div>

{#if opened && replaceMessage}
  <ConfirmDialog
    title="Replace everything?"
    message={replaceMessage}
    confirmLabel="Replace everything"
    cancelFirst
    onconfirm={replaceEverything}
    oncancel={keepCurrent}
  />
{/if}

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
    padding: 18px 0 10px;
  }
  h2 {
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .segmented {
    align-self: flex-start;
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
  .path {
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
  }
  .about {
    margin: 0;
    color: var(--text-dim);
    user-select: text;
    -webkit-user-select: text;
  }
  .shortcuts {
    display: grid;
    grid-template-columns: 1fr auto 72px;
    align-items: center;
    gap: 6px 12px;
  }
  .keys {
    min-width: 104px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--line-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-input);
    font-family: var(--mono);
    font-size: 12.5px;
    line-height: 28px;
    text-align: center;
    white-space: nowrap;
    cursor: pointer;
  }
  .keys:hover {
    border-color: var(--text-faint);
  }
  .keys.recording {
    border-color: var(--accent);
    color: var(--accent);
    box-shadow: 0 0 0 3px rgba(35, 160, 243, 0.2);
  }
  .keys.fixed {
    border-color: transparent;
    background: transparent;
    color: var(--text-dim);
    cursor: default;
  }
  .wallets {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .wallet {
    display: grid;
    grid-template-columns: 120px 1fr auto;
    align-items: center;
    column-gap: 10px;
    row-gap: 8px;
  }
  .wallet-name {
    color: var(--text-dim);
    font-size: 13px;
  }
  .address {
    font-family: var(--mono);
    font-size: 12.5px;
    overflow-wrap: anywhere;
    user-select: all;
    -webkit-user-select: all;
  }
  .qr {
    grid-column: 2;
    border-radius: var(--radius-sm);
  }
</style>
