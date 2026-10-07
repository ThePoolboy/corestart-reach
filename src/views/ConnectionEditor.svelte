<script lang="ts">
  import { homeDir, join } from '@tauri-apps/api/path';
  import { open } from '@tauri-apps/plugin-dialog';
  import { untrack } from 'svelte';
  import {
    api,
    errorText,
    type Connection,
    type Protocol,
    type RdpScreen,
    type Saved,
    type SecretUpdate,
    type Tree,
  } from '../lib/api';
  import { folderOptions } from '../lib/tree';
  import Icon from '../lib/Icon.svelte';
  import { pressed } from '../lib/shortcuts';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import SecretField from './SecretField.svelte';

  /** Common screen sizes for a fixed-size RDP screen. */
  const SIZES = [
    [1024, 768],
    [1280, 720],
    [1280, 800],
    [1280, 1024],
    [1366, 768],
    [1440, 900],
    [1600, 900],
    [1680, 1050],
    [1920, 1080],
    [1920, 1200],
    [2560, 1440],
    [3840, 2160],
  ].map(([w, h]) => `${w}x${h}`);

  /** What each screen mode does, shown as the buttons' tooltips. */
  const SCREEN_TIPS: Record<RdpScreen, string> = {
    window: 'The remote desktop resizes to fit the window',
    fixed: 'The remote desktop keeps the same size when you resize the window',
    fullscreen: 'The remote desktop fills the whole screen',
  };

  let {
    tree,
    connection,
    defaults,
    ondirty,
    onsaved,
    onconnect,
    onduplicate,
    ondelete,
    oncancel,
    oneditcredential,
  }: {
    tree: Tree;
    /** null when creating a new connection */
    connection: Connection | null;
    defaults: { protocol: Protocol; folderId: string | null };
    ondirty: (dirty: boolean) => void;
    onsaved: (saved: Saved) => void;
    onconnect: (id: string) => void;
    onduplicate: (id: string) => void;
    ondelete: (c: Connection) => void;
    oncancel: () => void;
    oneditcredential: (id: string) => void;
  } = $props();

  // The parent remounts this editor for each connection, so capture it once.
  const start = untrack(() => {
    const c = connection;
    return {
      name: c?.name ?? '',
      protocol: c?.protocol ?? defaults.protocol,
      host: c?.host ?? '',
      port: c?.port ? String(c.port) : '',
      folderId: c?.folderId ?? defaults.folderId ?? '',
      credentialId: c?.credentialId ?? '',
      username: c?.username ?? '',
      domain: c?.domain ?? '',
      sshKeyPath: c?.sshKeyPath ?? '',
      rdpScreen: (c?.rdpScreen ?? 'window') as RdpScreen,
      rdpWidth: String(c?.rdpSize.width ?? 1920),
      rdpHeight: String(c?.rdpSize.height ?? 1080),
      notes: c?.notes ?? '',
    };
  });
  const isNew = untrack(() => connection === null);

  let form = $state({ ...start });
  let password = $state<SecretUpdate>('keep');
  let passphrase = $state<SecretUpdate>('keep');
  let error = $state('');
  let saving = $state(false);

  const dirty = $derived(
    JSON.stringify(form) !== JSON.stringify(start) || password !== 'keep' || passphrase !== 'keep',
  );
  $effect(() => ondirty(dirty));

  /** Bumped by Revert to reset the password fields. */
  let resets = $state(0);
  const folders = $derived(folderOptions(tree.folders));
  const credential = $derived(tree.credentials.find((c) => c.id === form.credentialId));
  const defaultPort = $derived(form.protocol === 'rdp' ? 3389 : 22);

  /** The size picker shows "Custom" with width and height boxes for any other size. */
  let customSize = $state(untrack(() => !SIZES.includes(`${start.rdpWidth}x${start.rdpHeight}`)));
  const sizeChoice = $derived(customSize ? 'custom' : `${form.rdpWidth}x${form.rdpHeight}`);

  function pickSize(choice: string) {
    customSize = choice === 'custom';
    if (!customSize) [form.rdpWidth, form.rdpHeight] = choice.split('x');
  }

  function revert() {
    form = { ...start };
    customSize = !SIZES.includes(`${start.rdpWidth}x${start.rdpHeight}`);
    password = 'keep';
    passphrase = 'keep';
    error = '';
    resets++;
  }

  async function save(e?: SubmitEvent) {
    e?.preventDefault();
    if (saving) return;
    error = '';
    const port = form.port.trim() ? Number(form.port.trim()) : null;
    if (port !== null && (!Number.isInteger(port) || port < 1 || port > 65535)) {
      error = 'Port must be a number from 1 to 65535.';
      return;
    }
    const rdpSize = { width: Number(form.rdpWidth.trim()), height: Number(form.rdpHeight.trim()) };
    const fixed = form.protocol === 'rdp' && form.rdpScreen === 'fixed';
    const between = (n: number, min: number, max: number) => Number.isInteger(n) && n >= min && n <= max;
    if (fixed && !(between(rdpSize.width, 640, 8192) && between(rdpSize.height, 480, 8192))) {
      error = 'Screen size must be from 640×480 to 8192×8192.';
      return;
    }
    saving = true;
    try {
      const saved = await api.saveConnection({
        id: connection?.id ?? null,
        name: form.name,
        folderId: form.folderId || null,
        protocol: form.protocol,
        host: form.host,
        port,
        credentialId: form.credentialId || null,
        username: form.username,
        domain: form.protocol === 'rdp' ? form.domain : '',
        password,
        sshKeyPath: form.protocol === 'ssh' ? form.sshKeyPath : '',
        sshKeyPassphrase: form.protocol === 'ssh' ? passphrase : 'clear',
        rdpScreen: form.rdpScreen,
        // Kept as it was unless it's in use, so switching modes doesn't lose it.
        rdpSize: fixed ? rdpSize : (connection?.rdpSize ?? { width: 1920, height: 1080 }),
        notes: form.notes,
      });
      onsaved(saved);
    } catch (err) {
      error = errorText(err);
    } finally {
      saving = false;
    }
  }

  /** Pick a key file with the system file picker, starting in ~/.ssh. In a
   *  Flatpak this is also how the sandbox gets to see a key kept elsewhere. */
  async function browseKey() {
    try {
      const start = await join(await homeDir(), '.ssh').catch(() => undefined);
      const picked = await open({ title: 'Choose a private key file', multiple: false, directory: false, defaultPath: start });
      if (typeof picked === 'string') form.sshKeyPath = picked;
    } catch (err) {
      error = errorText(err);
    }
  }

  function keydown(e: KeyboardEvent) {
    if (pressed(e, tree.settings, 'save')) {
      e.preventDefault();
      if (isNew || dirty) save();
    }
  }

  /** The "⋯" menu: the rarely used actions, out of the header. */
  let menu = $state<{ x: number; y: number } | null>(null);
  let moreButton = $state<HTMLButtonElement>();
  const menuItems = $derived<MenuItem[]>(
    connection
      ? [
          { label: 'Duplicate', icon: 'copy', action: () => onduplicate(connection.id) },
          'separator',
          { label: 'Delete', icon: 'trash', danger: true, action: () => ondelete(connection) },
        ]
      : [],
  );

  function toggleMenu(e: MouseEvent) {
    if (menu) return closeMenu();
    // Right-aligned under the button (the menu is 200px wide).
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: r.right - 200, y: r.bottom + 4 };
  }

  function closeMenu() {
    menu = null;
    moreButton?.focus();
  }

  function focus(node: HTMLInputElement) {
    if (isNew) node.focus();
  }
</script>

<svelte:window onkeydown={keydown} />

<form class="editor" onsubmit={save}>
  <header>
    <div class="title">
      <span class="proto {form.protocol}">
        <Icon name={form.protocol === 'rdp' ? 'monitor' : 'terminal'} size={20} />
      </span>
      <h1>{isNew ? `New ${form.protocol.toUpperCase()} connection` : connection?.name}</h1>
    </div>
    {#if connection}
      <div class="actions">
        <!-- While the menu is open, pressing the button closes it rather than
             letting the menu's outside-click close it and the click reopen it. -->
        <button
          type="button"
          class="btn ghost icon"
          title="More actions"
          aria-label="More actions"
          aria-haspopup="menu"
          aria-expanded={menu !== null}
          bind:this={moreButton}
          onmousedown={(e) => menu && e.stopPropagation()}
          onclick={toggleMenu}
        >
          <Icon name="more" />
        </button>
        <button
          type="button"
          class="btn primary large"
          onclick={() => onconnect(connection.id)}
          title={dirty ? 'Save your changes first' : 'Connect'}
          disabled={dirty}
        >
          <Icon name="play" /> Connect
        </button>
      </div>
    {/if}
  </header>

  <div class="scroll">
    <section>
      <h2>Connection</h2>
      <div class="row">
        <label class="field">
          <span>Name</span>
          <input class="input" bind:value={form.name} placeholder={form.host || 'e.g. Web server 1'} />
        </label>
        <label class="field">
          <span>Folder</span>
          <select class="input" bind:value={form.folderId}>
            <option value="">(No folder)</option>
            {#each folders as f (f.id)}
              <option value={f.id}>{f.label}</option>
            {/each}
          </select>
        </label>
      </div>
      <div class="row address">
        <label class="field">
          <span>Protocol</span>
          <select class="input" bind:value={form.protocol}>
            <option value="rdp">RDP</option>
            <option value="ssh">SSH</option>
          </select>
        </label>
        <label class="field">
          <span>Host</span>
          <input class="input" bind:value={form.host} placeholder="server.example.com or 10.0.0.5" spellcheck="false" use:focus />
        </label>
        <label class="field">
          <span>Port</span>
          <input class="input" bind:value={form.port} placeholder={String(defaultPort)} inputmode="numeric" />
        </label>
      </div>
    </section>

    <section>
      <h2>Login</h2>
      <label class="field">
        <span>Credentials</span>
        <div class="with-button">
          <select class="input" bind:value={form.credentialId}>
            <option value="">Enter below</option>
            {#each tree.credentials as c (c.id)}
              <option value={c.id}>
                {c.name}{c.username ? ` (${c.domain ? c.domain + '\\' : ''}${c.username})` : ''}
              </option>
            {/each}
          </select>
          {#if credential}
            <button type="button" class="btn" title="Edit this saved credential" onclick={() => oneditcredential(credential.id)}>
              Edit
            </button>
          {/if}
        </div>
      </label>

      {#if !credential}
        <div class="row">
          <label class="field">
            <span>Username</span>
            <input class="input" bind:value={form.username} spellcheck="false" autocomplete="off" />
          </label>
          {#if form.protocol === 'rdp'}
            <label class="field">
              <span>Domain</span>
              <input class="input" bind:value={form.domain} placeholder="optional" spellcheck="false" />
            </label>
          {/if}
        </div>
        {#key resets}
          <SecretField
            label="Password"
            saved={connection?.hasPassword ?? false}
            bind:value={password}
            placeholder="Ask when connecting"
          />
        {/key}
      {/if}

      {#if form.protocol === 'ssh'}
        <label class="field">
          <span>Private key file</span>
          <div class="with-button">
            <input class="input" bind:value={form.sshKeyPath} placeholder="Optional: ssh-agent and ~/.ssh keys are tried" spellcheck="false" />
            <button type="button" class="btn" onclick={browseKey}>Browse…</button>
          </div>
        </label>
        {#if form.sshKeyPath.trim()}
          {#key resets}
            <SecretField
              label="Key passphrase"
              saved={connection?.hasKeyPassphrase ?? false}
              bind:value={passphrase}
              placeholder="None, or ask when connecting"
            />
          {/key}
        {/if}
      {/if}
    </section>

    {#if form.protocol === 'rdp'}
      <section>
        <h2>Display</h2>
        <div class="field">
          <span>Screen</span>
          <div class="screen">
            <div class="segmented">
              <button type="button" title={SCREEN_TIPS.window} class:on={form.rdpScreen === 'window'} onclick={() => (form.rdpScreen = 'window')}>
                Window
              </button>
              <button type="button" title={SCREEN_TIPS.fixed} class:on={form.rdpScreen === 'fixed'} onclick={() => (form.rdpScreen = 'fixed')}>
                Fixed size
              </button>
              <button type="button" title={SCREEN_TIPS.fullscreen} class:on={form.rdpScreen === 'fullscreen'} onclick={() => (form.rdpScreen = 'fullscreen')}>
                Full screen
              </button>
            </div>
            {#if form.rdpScreen === 'fixed'}
              <select class="input size" aria-label="Screen size" value={sizeChoice} onchange={(e) => pickSize(e.currentTarget.value)}>
                {#each SIZES as size (size)}
                  <option value={size}>{size.replace('x', ' × ')}</option>
                {/each}
                <option value="custom">Custom…</option>
              </select>
              {#if customSize}
                <input class="input num" aria-label="Width" bind:value={form.rdpWidth} inputmode="numeric" placeholder="1920" />
                <span class="times">×</span>
                <input class="input num" aria-label="Height" bind:value={form.rdpHeight} inputmode="numeric" placeholder="1080" />
              {/if}
            {/if}
          </div>
        </div>
      </section>
    {/if}

    <section>
      <h2>Notes</h2>
      <textarea class="input" bind:value={form.notes} placeholder="Anything worth remembering about this server"></textarea>
    </section>
  </div>

  {#if isNew || dirty}
    <footer>
      {#if error}
        <span class="error-text">{error}</span>
      {/if}
      <span class="spacer"></span>
      {#if isNew}
        <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      {:else}
        <button type="button" class="btn" onclick={revert}>Revert</button>
      {/if}
      <button type="submit" class="btn primary" disabled={saving}>{saving ? 'Saving…' : 'Save'}</button>
    </footer>
  {/if}
</form>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menuItems} onclose={closeMenu} />
{/if}

<style>
  .editor {
    height: 100%;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 28px 16px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .proto {
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
  }
  .proto.rdp {
    color: var(--rdp);
    background: color-mix(in srgb, var(--rdp) 14%, transparent);
  }
  .proto.ssh {
    color: var(--ssh);
    background: color-mix(in srgb, var(--ssh) 14%, transparent);
  }
  h1 {
    margin: 0;
    font-size: 20px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
    flex: none;
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 6px 28px 28px;
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
  .row.address {
    grid-template-columns: 96px 1fr 110px;
  }
  .with-button {
    display: flex;
    gap: 6px;
  }
  .with-button .input {
    flex: 1;
  }
  .screen {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .screen .size {
    width: 150px;
  }
  .screen .num {
    width: 76px;
  }
  .times {
    color: var(--text-faint);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 28px;
    border-top: 1px solid var(--line);
    background: var(--bg-side);
  }
  .spacer {
    flex: 1;
  }
</style>
