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
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      if (isNew || dirty) save();
    }
  }

  function focus(node: HTMLInputElement) {
    if (isNew) node.focus();
  }
</script>

<svelte:window onkeydown={keydown} />

<form class="editor" onsubmit={save}>
  <header>
    <div class="title">
      <span class="badge {form.protocol}">{form.protocol}</span>
      <div class="names">
        <h1>{isNew ? `New ${form.protocol.toUpperCase()} connection` : connection?.name}</h1>
        {#if !isNew && connection}
          <div class="sub">{connection.host}:{connection.port ?? defaultPort}</div>
        {/if}
      </div>
    </div>
    {#if connection}
      <div class="actions">
        <button type="button" class="btn ghost icon" title="Duplicate" onclick={() => onduplicate(connection.id)}>
          <Icon name="copy" />
        </button>
        <button type="button" class="btn ghost icon danger" title="Delete" onclick={() => ondelete(connection)}>
          <Icon name="trash" />
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
        <div class="field">
          <span>Protocol</span>
          <div class="segmented">
            <button type="button" class:on={form.protocol === 'rdp'} onclick={() => (form.protocol = 'rdp')}>RDP</button>
            <button type="button" class:on={form.protocol === 'ssh'} onclick={() => (form.protocol = 'ssh')}>SSH</button>
          </div>
        </div>
      </div>
      <div class="row host">
        <label class="field">
          <span>Host</span>
          <input class="input" bind:value={form.host} placeholder="server.example.com or 10.0.0.5" spellcheck="false" use:focus />
        </label>
        <label class="field">
          <span>Port</span>
          <input class="input" bind:value={form.port} placeholder={String(defaultPort)} inputmode="numeric" />
        </label>
      </div>
      <label class="field">
        <span>Folder</span>
        <select class="input" bind:value={form.folderId}>
          <option value="">(No folder)</option>
          {#each folders as f (f.id)}
            <option value={f.id}>{f.label}</option>
          {/each}
        </select>
      </label>
    </section>

    <section>
      <h2>Login</h2>
      <label class="field">
        <span>Credentials</span>
        <select class="input" bind:value={form.credentialId}>
          <option value="">Enter below</option>
          {#each tree.credentials as c (c.id)}
            <option value={c.id}>{c.name}{c.username ? ` (${c.username})` : ''}</option>
          {/each}
        </select>
      </label>

      {#if credential}
        <p class="using">
          <Icon name="key" /> Uses the saved credential <strong>{credential.name}</strong>
          {credential.username ? `(${credential.domain ? credential.domain + '\\' : ''}${credential.username})` : ''}.
          Edit it under Credentials.
        </p>
      {:else}
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
            hint="Leave empty to be asked each time you connect."
          />
        {/key}
      {/if}

      {#if form.protocol === 'ssh'}
        <label class="field">
          <span>Private key file</span>
          <div class="with-button">
            <input class="input" bind:value={form.sshKeyPath} placeholder="Optional. Without one, your ssh-agent and ~/.ssh keys are tried." spellcheck="false" />
            <button type="button" class="btn" onclick={browseKey}>Browse…</button>
          </div>
        </label>
        {#if form.sshKeyPath.trim()}
          {#key resets}
            <SecretField
              label="Key passphrase"
              saved={connection?.hasKeyPassphrase ?? false}
              bind:value={passphrase}
              hint="Leave empty if the key has none, or to be asked when connecting."
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
          <div class="segmented">
            <button type="button" class:on={form.rdpScreen === 'window'} onclick={() => (form.rdpScreen = 'window')}>
              Window
            </button>
            <button type="button" class:on={form.rdpScreen === 'fixed'} onclick={() => (form.rdpScreen = 'fixed')}>
              Fixed size
            </button>
            <button type="button" class:on={form.rdpScreen === 'fullscreen'} onclick={() => (form.rdpScreen = 'fullscreen')}>
              Full screen
            </button>
          </div>
          <span class="hint">
            {#if form.rdpScreen === 'window'}
              The remote desktop resizes to fit when you resize the window.
            {:else if form.rdpScreen === 'fixed'}
              The remote desktop keeps this size. Resizing the window scales the picture to fit.
            {:else}
              The remote desktop fills the screen.
            {/if}
          </span>
        </div>
        {#if form.rdpScreen === 'fixed'}
          <div class="row size">
            <label class="field">
              <span>Size</span>
              <select class="input" value={sizeChoice} onchange={(e) => pickSize(e.currentTarget.value)}>
                {#each SIZES as size (size)}
                  <option value={size}>{size.replace('x', ' × ')}</option>
                {/each}
                <option value="custom">Custom…</option>
              </select>
            </label>
            {#if customSize}
              <label class="field">
                <span>Width</span>
                <input class="input" bind:value={form.rdpWidth} inputmode="numeric" placeholder="1920" />
              </label>
              <label class="field">
                <span>Height</span>
                <input class="input" bind:value={form.rdpHeight} inputmode="numeric" placeholder="1080" />
              </label>
            {/if}
          </div>
        {/if}
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
    padding: 20px 28px 16px;
    border-bottom: 1px solid var(--line);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .names {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 20px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    color: var(--text-dim);
    font-family: var(--mono);
    font-size: 12.5px;
    user-select: text;
    -webkit-user-select: text;
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
  .row.size {
    grid-template-columns: 1fr 110px 110px;
  }
  .with-button {
    display: flex;
    gap: 6px;
  }
  .with-button .input {
    flex: 1;
  }
  .using {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    background: var(--bg-hover);
    color: var(--text-dim);
  }
  .using :global(svg) {
    color: var(--accent);
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
