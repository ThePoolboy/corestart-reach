<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Tree, type VaultStatus } from '../lib/api';
  import { toastError } from '../lib/toast.svelte';
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
