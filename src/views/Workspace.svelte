<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import {
    api,
    type Connection,
    type ConnectOutcome,
    type Credential,
    type Folder,
    type Login,
    type Protocol,
    type Saved,
    type Tree,
  } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import { keysFor, pressed } from '../lib/shortcuts';
  import { toast, toastError } from '../lib/toast.svelte';
  import { isInside, searchRows, visibleRows, type Row } from '../lib/tree';
  import { autoCheck, installUpdate } from '../lib/update.svelte';
  import ConfirmDialog from './ConfirmDialog.svelte';
  import ContextMenu, { type MenuItem } from './ContextMenu.svelte';
  import MoveDialog from './MoveDialog.svelte';
  import ConnectionEditor from './ConnectionEditor.svelte';
  import CredentialPrompt from './CredentialPrompt.svelte';
  import CredentialsPane from './CredentialsPane.svelte';
  import FolderPane from './FolderPane.svelte';
  import NameDialog from './NameDialog.svelte';
  import QuickConnect from './QuickConnect.svelte';
  import SettingsPane from './SettingsPane.svelte';
  import UpdateBanner from './UpdateBanner.svelte';

  let {
    tree,
    onchange,
    onlock,
    onrestore,
  }: { tree: Tree; onchange: (t: Tree) => void; onlock: () => void; onrestore: (t: Tree) => void } = $props();

  type View =
    | { kind: 'home' }
    | { kind: 'connection'; id: string }
    | { kind: 'new'; protocol: Protocol; folderId: string | null }
    | { kind: 'folder'; id: string }
    | { kind: 'credentials'; id: string | null }
    | { kind: 'settings' };

  type Dialog =
    | { kind: 'confirm'; title: string; message: string; label: string; run: () => void | Promise<void> }
    | { kind: 'folder'; parentId: string | null }
    | { kind: 'login'; name: string; username: string; domain: string; retry: (login: Login) => void }
    | { kind: 'quick'; initial: string }
    | { kind: 'move'; item: Item; name: string; current: string | null }
    | { kind: 'rename'; folder: Folder };

  /** Something in the tree that can be moved. */
  type Item = { kind: 'connection' | 'folder'; id: string };

  let view = $state<View>({ kind: 'home' });
  let dialog = $state<Dialog | null>(null);
  let query = $state('');
  let menuOpen = $state(false);
  let dirty = $state(false);
  /** Bumped after every save so the open editor remounts with fresh data. */
  let revision = $state(0);
  /** Open right-click menu, and the row it belongs to. */
  let ctx = $state<{ x: number; y: number; items: MenuItem[]; for: string | null } | null>(null);
  /** Item being dragged, and the folder it would drop into ('' = top level). */
  let dragging = $state<Item | null>(null);
  let dropTarget = $state<string | null>(null);
  let expandTimer: ReturnType<typeof setTimeout> | undefined;

  const OPEN_KEY = 'reach.openFolders';
  const open = new SvelteSet<string>(loadOpen());

  function loadOpen(): string[] {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(OPEN_KEY) ?? '[]');
      return Array.isArray(saved) ? saved.filter((id): id is string => typeof id === 'string') : [];
    } catch {
      return [];
    }
  }

  $effect(() => {
    try {
      localStorage.setItem(OPEN_KEY, JSON.stringify([...open]));
    } catch {
      // storage unavailable; folders just start collapsed next time
    }
  });

  const rows = $derived(
    query.trim()
      ? searchRows(tree.folders, tree.connections, query)
      : visibleRows(tree.folders, tree.connections, open),
  );
  const selectedId = $derived(
    view.kind === 'connection' || view.kind === 'folder' ? view.id : null,
  );
  const selectedConnection = $derived.by(() => {
    const v = view;
    return v.kind === 'connection' ? tree.connections.find((c) => c.id === v.id) : undefined;
  });
  const selectedFolder = $derived.by(() => {
    const v = view;
    return v.kind === 'folder' ? tree.folders.find((f) => f.id === v.id) : undefined;
  });
  /** Folder that "New …" should put things in. */
  const currentFolder = $derived(
    view.kind === 'folder' ? view.id : (selectedConnection?.folderId ?? null),
  );

  onMount(() => {
    const unlisten = listen<{ name: string; message: string }>('rdp-exit', (e) => {
      toast(`${e.payload.name}: ${e.payload.message}`, 'error');
    });
    return () => {
      unlisten.then((f) => f());
    };
  });

  // Look for a new version now and then (at most every 12 hours) while unlocked.
  onMount(() => {
    const check = () => {
      if (tree.settings.checkForUpdates) autoCheck();
    };
    check();
    const timer = setInterval(check, 60 * 60 * 1000);
    return () => clearInterval(timer);
  });

  /** Installing restarts Reach, which closes SSH windows and drops unsaved edits. */
  async function update() {
    const ssh = await api.sshWindowCount().catch(() => 0);
    const losses = [
      ssh ? `close ${ssh === 1 ? 'the open SSH window' : `${ssh} open SSH windows`}` : '',
      dirty ? 'discard your unsaved changes' : '',
    ].filter(Boolean);
    const start = () => installUpdate().catch(toastError);
    if (!losses.length) return start();
    dialog = {
      kind: 'confirm',
      title: 'Update and restart?',
      message: `Restarting Reach will ${losses.join(' and ')}. RDP sessions keep running.`,
      label: 'Update and restart',
      run: () => {
        dirty = false;
        start();
      },
    };
  }

  /** "14 connections in 5 folders" for the Home screen. */
  const summary = $derived.by(() => {
    const count = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`;
    const connections = count(tree.connections.length, 'connection');
    return tree.folders.length ? `${connections} in ${count(tree.folders.length, 'folder')}` : connections;
  });

  // ---- navigation -------------------------------------------------------

  function go(next: View) {
    menuOpen = false;
    if (dirty) {
      dialog = {
        kind: 'confirm',
        title: 'Discard changes?',
        message: 'You have unsaved changes. Leave without saving them?',
        label: 'Discard',
        run: () => {
          dirty = false;
          view = next;
        },
      };
      return;
    }
    view = next;
  }

  function reveal(folderId: string | null) {
    const seen = new Set<string>();
    let cursor = tree.folders.find((f) => f.id === folderId);
    while (cursor && !seen.has(cursor.id)) {
      seen.add(cursor.id);
      open.add(cursor.id);
      cursor = tree.folders.find((f) => f.id === cursor!.parentId);
    }
  }

  function applySaved(saved: Saved, next: View) {
    onchange(saved.tree);
    dirty = false;
    revision++;
    view = next;
  }

  // ---- actions ----------------------------------------------------------

  /** Launch, asking for a login first if there's nothing to log in with. */
  async function launch(
    name: string,
    protocol: Protocol,
    attempt: (login?: Login) => Promise<ConnectOutcome>,
    login?: Login,
  ) {
    try {
      const result = await attempt(login);
      if (result.status === 'needCredentials') {
        dialog = {
          kind: 'login',
          name,
          username: result.username,
          domain: result.domain,
          retry: (l) => launch(name, protocol, attempt, l),
        };
      } else if (protocol === 'rdp') {
        toast(`Opening ${name}…`);
      }
    } catch (e) {
      toastError(e);
    }
  }

  function connect(id: string) {
    const c = tree.connections.find((x) => x.id === id);
    if (c) launch(c.name, c.protocol, (login) => api.connect(id, login));
  }

  function quickConnect(protocol: Protocol, address: string) {
    dialog = null;
    launch(address, protocol, (login) => api.quickConnect(protocol, address, login));
  }

  function openQuickConnect(initial = '') {
    menuOpen = false;
    dialog = { kind: 'quick', initial };
  }

  function newConnection(protocol: Protocol, folderId: string | null = currentFolder) {
    go({ kind: 'new', protocol, folderId });
  }

  function askNewFolder(parentId: string | null = currentFolder) {
    menuOpen = false;
    dialog = { kind: 'folder', parentId };
  }

  async function createFolder(name: string, parentId: string | null) {
    try {
      const saved = await api.saveFolder({ id: null, name, parentId });
      dialog = null;
      reveal(parentId);
      applySaved(saved, { kind: 'folder', id: saved.id });
    } catch (e) {
      toastError(e);
    }
  }

  function confirmDeleteConnection(c: Connection) {
    dialog = {
      kind: 'confirm',
      title: `Delete ${c.name}?`,
      message: 'The connection and any password saved with it will be removed.',
      label: 'Delete',
      run: async () => {
        onchange(await api.deleteConnection(c.id));
        dirty = false;
        view = { kind: 'home' };
      },
    };
  }

  async function duplicate(id: string) {
    try {
      const saved = await api.duplicateConnection(id);
      applySaved(saved, { kind: 'connection', id: saved.id });
      toast('Connection duplicated.');
    } catch (e) {
      toastError(e);
    }
  }

  function confirmDeleteFolder(f: Folder) {
    const parent = tree.folders.find((x) => x.id === f.parentId);
    dialog = {
      kind: 'confirm',
      title: `Delete folder ${f.name}?`,
      message: `Anything inside it moves to ${parent ? `"${parent.name}"` : 'the top level'}. No connections are deleted.`,
      label: 'Delete folder',
      run: async () => {
        onchange(await api.deleteFolder(f.id));
        dirty = false;
        view = parent ? { kind: 'folder', id: parent.id } : { kind: 'home' };
      },
    };
  }

  function confirmDeleteCredential(c: Credential, usedBy: number) {
    dialog = {
      kind: 'confirm',
      title: `Delete credential ${c.name}?`,
      message: usedBy
        ? `${usedBy} connection${usedBy === 1 ? ' uses' : 's use'} it. They will go back to their own username and password fields.`
        : 'No connections use it.',
      label: 'Delete',
      run: async () => {
        onchange(await api.deleteCredential(c.id));
        dirty = false;
        view = { kind: 'credentials', id: null };
      },
    };
  }

  async function runConfirm(run: () => void | Promise<void>) {
    try {
      await run();
      dialog = null;
    } catch (e) {
      toastError(e);
    }
  }

  // ---- tree interaction -------------------------------------------------

  function clickRow(row: Row) {
    if (row.kind === 'folder') {
      if (selectedId === row.folder.id) {
        if (open.has(row.folder.id)) open.delete(row.folder.id);
        else open.add(row.folder.id);
      } else {
        go({ kind: 'folder', id: row.folder.id });
      }
    } else {
      go({ kind: 'connection', id: row.connection.id });
    }
  }

  function toggle(e: MouseEvent, id: string) {
    e.stopPropagation();
    if (open.has(id)) open.delete(id);
    else open.add(id);
  }

  function rowId(row: Row) {
    return row.kind === 'folder' ? row.folder.id : row.connection.id;
  }

  /** Arrow keys move through the tree, Enter connects, Left/Right fold. */
  function treeKey(e: KeyboardEvent) {
    const index = rows.findIndex((r) => rowId(r) === selectedId);
    const row = rows[index];
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = rows[Math.max(0, Math.min(rows.length - 1, index + (e.key === 'ArrowDown' ? 1 : -1)))];
      if (next) clickRowSelectOnly(next);
    } else if (e.key === 'Enter' && row?.kind === 'connection') {
      e.preventDefault();
      connect(row.connection.id);
    } else if (e.key === 'ArrowRight' && row?.kind === 'folder') {
      open.add(row.folder.id);
    } else if (e.key === 'ArrowLeft' && row?.kind === 'folder') {
      open.delete(row.folder.id);
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      keyMenu(e);
    }
  }

  // ---- moving: drag and drop, "Move to…" ----------------------------------

  function folderOf(item: Item): string | null {
    return item.kind === 'folder'
      ? (tree.folders.find((f) => f.id === item.id)?.parentId ?? null)
      : (tree.connections.find((c) => c.id === item.id)?.folderId ?? null);
  }

  /** Can `item` go into `folderId` (null = top level)? */
  function canMove(item: Item, folderId: string | null) {
    if (folderOf(item) === folderId) return false;
    return item.kind === 'connection' || !isInside(tree.folders, folderId, item.id);
  }

  async function moveItem(item: Item, folderId: string | null) {
    if (!canMove(item, folderId)) return;
    if (dirty && selectedId === item.id) {
      toast('Save or revert your changes before moving this.', 'error');
      return;
    }
    try {
      onchange(await api.moveItem(item.kind, item.id, folderId));
      reveal(folderId);
      if (selectedId === item.id) revision++; // editor shows the new folder
    } catch (e) {
      toastError(e);
    }
  }

  function itemOf(row: Row): Item {
    return row.kind === 'folder' ? { kind: 'folder', id: row.folder.id } : { kind: 'connection', id: row.connection.id };
  }

  function dragStart(e: DragEvent, row: Row) {
    dragging = itemOf(row);
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      // WebKit only starts a drag when some data is set.
      e.dataTransfer.setData('text/plain', row.kind === 'folder' ? row.folder.name : row.connection.name);
    }
  }

  function dragEnd() {
    dragging = null;
    dropTarget = null;
    clearTimeout(expandTimer);
  }

  /** Hovering `folderId` ('' = top level) during a drag. */
  function dragOver(e: DragEvent, folderId: string) {
    e.stopPropagation();
    if (!dragging || !canMove(dragging, folderId || null)) {
      if (e.dataTransfer) e.dataTransfer.dropEffect = 'none';
      return;
    }
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    if (dropTarget !== folderId) {
      dropTarget = folderId;
      // Hold over a closed folder to open it.
      clearTimeout(expandTimer);
      if (folderId && !open.has(folderId)) expandTimer = setTimeout(() => open.add(folderId), 700);
    }
  }

  function drop(e: DragEvent, folderId: string) {
    e.preventDefault();
    e.stopPropagation();
    const item = dragging;
    dragEnd();
    if (item) moveItem(item, folderId || null);
  }

  /** Where dropping onto this row puts things: into a folder, or beside a connection. */
  function rowTarget(row: Row): string {
    return row.kind === 'folder' ? row.folder.id : (row.connection.folderId ?? '');
  }

  // ---- right-click menus --------------------------------------------------

  function askMove(item: Item) {
    const name =
      item.kind === 'folder'
        ? (tree.folders.find((f) => f.id === item.id)?.name ?? '')
        : (tree.connections.find((c) => c.id === item.id)?.name ?? '');
    dialog = { kind: 'move', item, name, current: folderOf(item) };
  }

  function rowMenu(e: MouseEvent, row: Row) {
    e.preventDefault();
    e.stopPropagation();
    const items: MenuItem[] =
      row.kind === 'connection'
        ? [
            { label: 'Connect', icon: 'play', action: () => connect(row.connection.id) },
            { label: 'Edit', action: () => go({ kind: 'connection', id: row.connection.id }) },
            'separator',
            { label: 'Move to…', icon: 'folder', action: () => askMove(itemOf(row)) },
            { label: 'Duplicate', icon: 'copy', action: () => duplicate(row.connection.id) },
            'separator',
            { label: 'Delete', icon: 'trash', danger: true, action: () => confirmDeleteConnection(row.connection) },
          ]
        : [
            { label: 'New RDP connection here', icon: 'monitor', action: () => newConnection('rdp', row.folder.id) },
            { label: 'New SSH connection here', icon: 'terminal', action: () => newConnection('ssh', row.folder.id) },
            { label: 'New subfolder', icon: 'folder', action: () => askNewFolder(row.folder.id) },
            'separator',
            { label: 'Rename…', action: () => (dialog = { kind: 'rename', folder: row.folder }) },
            { label: 'Move to…', action: () => askMove(itemOf(row)) },
            'separator',
            { label: 'Delete folder', icon: 'trash', danger: true, action: () => confirmDeleteFolder(row.folder) },
          ];
    ctx = { x: e.clientX, y: e.clientY, items, for: rowId(row) };
  }

  function treeMenu(e: MouseEvent) {
    e.preventDefault();
    ctx = {
      x: e.clientX,
      y: e.clientY,
      for: null,
      items: [
        { label: 'Quick connect…', icon: 'bolt', action: () => openQuickConnect() },
        'separator',
        { label: 'New RDP connection', icon: 'monitor', action: () => newConnection('rdp', null) },
        { label: 'New SSH connection', icon: 'terminal', action: () => newConnection('ssh', null) },
        { label: 'New folder', icon: 'folder', action: () => askNewFolder(null) },
      ],
    };
  }

  /** Shift+F10 or the Menu key opens the menu for the selected row. */
  function keyMenu(e: KeyboardEvent) {
    const row = rows.find((r) => rowId(r) === selectedId);
    const el = document.querySelector('.tree .on');
    if (!row || !el) return;
    const r = el.getBoundingClientRect();
    rowMenu(new MouseEvent('contextmenu', { clientX: r.left + 24, clientY: r.bottom }), row);
    e.preventDefault();
  }

  async function renameFolder(folder: Folder, name: string) {
    try {
      const saved = await api.saveFolder({ id: folder.id, name, parentId: folder.parentId });
      dialog = null;
      onchange(saved.tree);
      if (selectedId === folder.id && !dirty) revision++;
    } catch (e) {
      toastError(e);
    }
  }

  function clickRowSelectOnly(row: Row) {
    go(row.kind === 'folder' ? { kind: 'folder', id: row.folder.id } : { kind: 'connection', id: row.connection.id });
    requestAnimationFrame(() => document.querySelector('.tree .on')?.scrollIntoView({ block: 'nearest' }));
  }

  /** The shortcuts set in Settings (Save is the editor's own). */
  function globalKey(e: KeyboardEvent) {
    const settings = tree.settings;
    if (pressed(e, settings, 'search')) {
      e.preventDefault();
      document.getElementById('search')?.focus();
    } else if (pressed(e, settings, 'lock')) {
      e.preventDefault();
      onlock();
    } else if (pressed(e, settings, 'settings')) {
      e.preventDefault();
      go({ kind: 'settings' });
    } else if (pressed(e, settings, 'quickConnect')) {
      e.preventDefault();
      openQuickConnect();
    } else if (pressed(e, settings, 'newConnection')) {
      e.preventDefault();
      newConnection(selectedConnection?.protocol ?? 'rdp');
    }
  }

  function closeMenu(e: MouseEvent) {
    if (menuOpen && !(e.target as HTMLElement).closest('.new-menu')) menuOpen = false;
  }
</script>

<svelte:window onkeydown={globalKey} onclick={closeMenu} />

<div class="layout">
  <aside>
    <div class="tools">
      <div class="search">
        <Icon name="search" size={14} />
        <input
          id="search"
          class="input"
          placeholder="Search ({keysFor(tree.settings, 'search')})"
          bind:value={query}
          spellcheck="false"
          onkeydown={(e) => {
            if (e.key === 'Escape') query = '';
            if (e.key !== 'Enter') return;
            if (rows[0]?.kind === 'connection') connect(rows[0].connection.id);
            else if (query.trim()) openQuickConnect(query.trim());
          }}
        />
      </div>
      <button class="btn icon" title="Quick connect ({keysFor(tree.settings, 'quickConnect')})" onclick={() => openQuickConnect()}>
        <Icon name="bolt" />
      </button>
      <div class="new-menu">
        <button class="btn primary icon" title="New…" onclick={() => (menuOpen = !menuOpen)}>
          <Icon name="plus" />
        </button>
        {#if menuOpen}
          <div class="menu" role="menu">
            <button role="menuitem" onclick={() => newConnection('rdp')}><Icon name="monitor" /> RDP connection</button>
            <button role="menuitem" onclick={() => newConnection('ssh')}><Icon name="terminal" /> SSH connection</button>
            <button role="menuitem" onclick={() => askNewFolder()}><Icon name="folder" /> Folder</button>
          </div>
        {/if}
      </div>
      <button class="btn icon" title="Lock Reach ({keysFor(tree.settings, 'lock')})" aria-label="Lock Reach" onclick={onlock}>
        <Icon name="lock" />
      </button>
    </div>

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <ul
      class="tree"
      class:drop-root={dropTarget === ''}
      tabindex="0"
      onkeydown={treeKey}
      oncontextmenu={treeMenu}
      ondragover={(e) => dragOver(e, '')}
      ondrop={(e) => drop(e, '')}
      ondragleave={(e) => {
        if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dropTarget = null;
      }}
      role="tree"
      aria-label="Connections"
    >
      {#each rows as row (rowId(row))}
        <li
          role="treeitem"
          aria-selected={selectedId === rowId(row)}
          draggable="true"
          ondragstart={(e) => dragStart(e, row)}
          ondragend={dragEnd}
          ondragover={(e) => dragOver(e, rowTarget(row))}
          ondrop={(e) => drop(e, rowTarget(row))}
          oncontextmenu={(e) => rowMenu(e, row)}
          class:dragged={dragging?.id === rowId(row)}
        >
          {#if row.kind === 'folder'}
            <button
              class="row folder"
              class:on={selectedId === row.folder.id}
              class:drop={dropTarget === row.folder.id}
              class:menu-on={ctx?.for === row.folder.id}
              style="padding-left: {10 + row.depth * 16}px"
              onclick={() => clickRow(row)}
              tabindex="-1"
            >
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <span class="chev" class:open={row.open} onclick={(e) => toggle(e, row.folder.id)}>
                <Icon name="chevron" size={14} />
              </span>
              <Icon name="folder" />
              <span class="name">{row.folder.name}</span>
              {#if !row.open}<span class="count">{row.count}</span>{/if}
            </button>
          {:else}
            <button
              class="row"
              class:on={selectedId === row.connection.id}
              class:menu-on={ctx?.for === row.connection.id}
              style="padding-left: {row.path !== undefined ? 10 : 32 + row.depth * 16}px"
              onclick={() => clickRow(row)}
              ondblclick={() => connect(row.connection.id)}
              title="{row.connection.host} (double-click to connect)"
              tabindex="-1"
            >
              <span
                class="proto {row.connection.protocol}"
                role="img"
                aria-label={row.connection.protocol.toUpperCase()}
                title={row.connection.protocol.toUpperCase()}
              >
                <Icon name={row.connection.protocol === 'rdp' ? 'monitor' : 'terminal'} />
              </span>
              <span class="name">
                {row.connection.name}
                {#if row.path}<small>{row.path}</small>{/if}
              </span>
            </button>
          {/if}
        </li>
      {:else}
        <li class="empty">
          {#if query.trim()}
            Nothing matches "{query.trim()}".<br />Press Enter to quick connect to it.
          {:else}
            Press <strong>+</strong> to add a connection.
          {/if}
        </li>
      {/each}
    </ul>

    <div class="foot">
      <button class="nav" class:on={view.kind === 'home'} onclick={() => go({ kind: 'home' })}>
        <Icon name="home" /> Home
      </button>
      <button class="nav" class:on={view.kind === 'credentials'} onclick={() => go({ kind: 'credentials', id: null })}>
        <Icon name="key" /> Credentials
      </button>
      <button class="nav" class:on={view.kind === 'settings'} onclick={() => go({ kind: 'settings' })}>
        <Icon name="settings" /> Settings
      </button>
    </div>
  </aside>

  <main>
    <UpdateBanner onupdate={update} />
    <div class="view">
      {#key `${view.kind}:${'id' in view ? view.id : ''}:${revision}`}
        {#if view.kind === 'connection' && selectedConnection}
          <ConnectionEditor
            {tree}
            connection={selectedConnection}
            defaults={{ protocol: selectedConnection.protocol, folderId: selectedConnection.folderId }}
            ondirty={(d) => (dirty = d)}
            onsaved={(s) => applySaved(s, { kind: 'connection', id: s.id })}
            onconnect={(id) => connect(id)}
            onduplicate={duplicate}
            ondelete={confirmDeleteConnection}
            oneditcredential={(id) => go({ kind: 'credentials', id })}
            oncancel={() => go({ kind: 'home' })}
          />
        {:else if view.kind === 'new'}
          <ConnectionEditor
            {tree}
            connection={null}
            defaults={{ protocol: view.protocol, folderId: view.folderId }}
            ondirty={(d) => (dirty = d)}
            onsaved={(s) => {
              reveal(s.tree.connections.find((c) => c.id === s.id)?.folderId ?? null);
              applySaved(s, { kind: 'connection', id: s.id });
            }}
            onconnect={(id) => connect(id)}
            onduplicate={duplicate}
            ondelete={confirmDeleteConnection}
            oneditcredential={(id) => go({ kind: 'credentials', id })}
            oncancel={() => {
              dirty = false;
              go({ kind: 'home' });
            }}
          />
        {:else if view.kind === 'folder' && selectedFolder}
          <FolderPane
            {tree}
            folder={selectedFolder}
            ondirty={(d) => (dirty = d)}
            onsaved={(s) => applySaved(s, { kind: 'folder', id: s.id })}
            ondelete={confirmDeleteFolder}
            onnew={(protocol, folderId) => newConnection(protocol, folderId)}
            onnewfolder={(parentId) => askNewFolder(parentId)}
          />
        {:else if view.kind === 'credentials'}
          <CredentialsPane
            {tree}
            selected={view.id}
            ondirty={(d) => (dirty = d)}
            onselect={(id) => go({ kind: 'credentials', id })}
            onsaved={(s) => applySaved(s, { kind: 'credentials', id: s.id })}
            ondelete={confirmDeleteCredential}
          />
        {:else if view.kind === 'settings'}
          <SettingsPane {tree} {onchange} {onrestore} />
        {:else}
          <div class="home">
            {#if tree.connections.length}
              <h1>{summary}</h1>
              <p>Double&#8209;click a connection to open it.</p>
            {:else}
              <h1>No connections yet</h1>
              <p>Add your first one, or quick connect to any host without saving it.</p>
            {/if}
            <div class="home-actions">
              <button class="btn primary" title="Quick connect ({keysFor(tree.settings, 'quickConnect')})" onclick={() => openQuickConnect()}>
                <Icon name="bolt" /> Quick connect
              </button>
              <!-- Like its shortcut: starts as RDP; the form's Protocol switches it to SSH. -->
              <button class="btn" title="New connection ({keysFor(tree.settings, 'newConnection')})" onclick={() => newConnection('rdp')}>
                <Icon name="plus" /> New connection
              </button>
            </div>
          </div>
        {/if}
      {/key}
    </div>
  </main>
</div>

{#if dialog?.kind === 'confirm'}
  {@const d = dialog}
  <ConfirmDialog
    title={d.title}
    message={d.message}
    confirmLabel={d.label}
    onconfirm={() => runConfirm(d.run)}
    oncancel={() => (dialog = null)}
  />
{:else if dialog?.kind === 'folder'}
  {@const parentId = dialog.parentId}
  <NameDialog
    title="New folder"
    label="Folder name"
    onsubmit={(name) => createFolder(name, parentId)}
    oncancel={() => (dialog = null)}
  />
{:else if dialog?.kind === 'login'}
  {@const d = dialog}
  <CredentialPrompt
    name={d.name}
    username={d.username}
    domain={d.domain}
    onsubmit={(login) => {
      // `d` follows `dialog`, so take what we need before closing it.
      const retry = d.retry;
      dialog = null;
      retry(login);
    }}
    oncancel={() => (dialog = null)}
  />
{:else if dialog?.kind === 'quick'}
  <QuickConnect initial={dialog.initial} onsubmit={quickConnect} oncancel={() => (dialog = null)} />
{:else if dialog?.kind === 'move'}
  {@const item = dialog.item}
  <MoveDialog
    name={dialog.name}
    folders={tree.folders}
    current={dialog.current}
    exclude={item.kind === 'folder' ? item.id : undefined}
    onsubmit={(folderId) => {
      // `item` follows `dialog`, so take it before closing the dialog.
      const target = item;
      dialog = null;
      moveItem(target, folderId);
    }}
    oncancel={() => (dialog = null)}
  />
{:else if dialog?.kind === 'rename'}
  {@const folder = dialog.folder}
  <NameDialog
    title="Rename folder"
    label="Folder name"
    initial={folder.name}
    confirmLabel="Rename"
    onsubmit={(name) => renameFolder(folder, name)}
    oncancel={() => (dialog = null)}
  />
{/if}

{#if ctx}
  <ContextMenu x={ctx.x} y={ctx.y} items={ctx.items} onclose={() => (ctx = null)} />
{/if}

<style>
  .layout {
    height: 100%;
    display: grid;
    grid-template-columns: 290px 1fr;
  }
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--line);
    background: var(--bg-side);
  }
  .tools {
    display: flex;
    gap: 6px;
    padding: 12px 10px 10px;
  }
  .search {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }
  .search :global(svg) {
    position: absolute;
    left: 10px;
    color: var(--text-faint);
    pointer-events: none;
  }
  .search .input {
    height: 32px;
    padding-left: 30px;
  }
  .new-menu {
    position: relative;
  }
  .menu {
    position: absolute;
    top: 38px;
    right: 0;
    z-index: 20;
    min-width: 190px;
    padding: 5px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg-raised);
    box-shadow: var(--shadow);
  }
  .menu button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .menu button:hover {
    background: var(--bg-hover);
  }
  .tree {
    flex: 1;
    margin: 0;
    padding: 2px 6px 8px;
    list-style: none;
    overflow-y: auto;
    outline: none;
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding-right: 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.on {
    background: var(--bg-selected);
  }
  .tree:focus-visible .row.on {
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .row.menu-on {
    box-shadow: inset 0 0 0 1px var(--line-strong);
  }
  /* drag and drop */
  .tree.drop-root {
    box-shadow: inset 0 0 0 2px var(--accent);
    border-radius: var(--radius-sm);
  }
  .row.drop {
    background: var(--bg-selected);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  li.dragged {
    opacity: 0.45;
  }
  .row.folder :global(svg) {
    flex: none;
    color: var(--text-dim);
  }
  .chev {
    display: grid;
    place-items: center;
    width: 14px;
    transition: transform 0.12s;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name small {
    margin-left: 6px;
    color: var(--text-faint);
  }
  .proto {
    flex: none;
    display: grid;
  }
  .proto.rdp {
    color: var(--rdp);
  }
  .proto.ssh {
    color: var(--ssh);
  }
  .count {
    font-size: 11px;
    color: var(--text-faint);
  }
  .empty {
    padding: 16px 10px;
    color: var(--text-faint);
    text-align: center;
    text-wrap: balance;
  }
  .foot {
    padding: 8px;
    border-top: 1px solid var(--line);
  }
  .nav {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    cursor: pointer;
    text-align: left;
  }
  .nav:hover {
    background: var(--bg-hover);
  }
  .nav.on {
    background: var(--bg-selected);
  }
  main {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .view {
    flex: 1;
    min-height: 0;
  }
  .home {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    /* A little above the middle, where centred content looks centred. */
    padding: 24px 24px 72px;
    text-align: center;
  }
  .home h1 {
    margin: 0;
    font-size: 22px;
  }
  .home p {
    margin: 0;
    color: var(--text-dim);
    max-width: 420px;
    text-wrap: balance;
  }
  .home-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 12px;
  }
</style>
