<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { api, type Tree, type VaultStatus } from '../lib/api';
  import { toast, toastError } from '../lib/toast.svelte';
  import LockScreen from './LockScreen.svelte';
  import Toasts from './Toasts.svelte';
  import Workspace from './Workspace.svelte';

  let status = $state<VaultStatus | null>(null);
  let tree = $state<Tree | null>(null);

  onMount(async () => {
    try {
      status = await api.vaultStatus();
      // Still unlocked after a page reload (e.g. during development).
      if (status.unlocked) tree = await api.getTree();
    } catch (e) {
      toastError(e);
    }
  });

  // Auto-lock: the Rust side locks the vault after the idle time in Settings
  // and tells us; we tell it whenever you use this window (at most every 20 s).
  onMount(() => {
    const unlisten = listen<number>('vault-locked', async (e) => {
      tree = null;
      status = await api.vaultStatus().catch(() => status);
      toast(`Locked after ${e.payload} minute${e.payload === 1 ? '' : 's'} without use.`);
    });

    let reported = 0;
    const active = () => {
      const now = Date.now();
      if (tree && now - reported > 20_000) {
        reported = now;
        api.vaultTouch().catch(() => {});
      }
    };
    const events = ['pointerdown', 'pointermove', 'keydown', 'wheel'] as const;
    for (const name of events) window.addEventListener(name, active, { passive: true });

    return () => {
      unlisten.then((stop) => stop());
      for (const name of events) window.removeEventListener(name, active);
    };
  });

  async function lock() {
    try {
      await api.vaultLock();
      tree = null;
      status = await api.vaultStatus();
    } catch (e) {
      toastError(e);
    }
  }
</script>

{#if status}
  {#if tree}
    <Workspace {tree} onchange={(t) => (tree = t)} onlock={lock} />
  {:else}
    <LockScreen {status} onunlock={(t) => (tree = t)} />
  {/if}
{/if}
<Toasts />
