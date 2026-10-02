<script lang="ts">
  import { api } from '../lib/api';
  import Icon from '../lib/Icon.svelte';
  import { toastError } from '../lib/toast.svelte';
  import { updates } from '../lib/update.svelte';

  let { onupdate }: { onupdate: () => void } = $props();
</script>

{#if updates.available && !updates.dismissed}
  <div class="banner" role="status">
    <Icon name="download" />
    <span class="text">
      {updates.installing
        ? `Downloading Corestart Reach ${updates.available.version}…`
        : `Corestart Reach ${updates.available.version} is available.`}
    </span>
    <button class="btn ghost" onclick={() => api.openLink('releases').catch(toastError)}>What's new</button>
    <button class="btn primary" disabled={updates.installing} onclick={onupdate}>Update and restart</button>
    <button
      class="btn ghost icon"
      title="Later"
      aria-label="Later"
      disabled={updates.installing}
      onclick={() => (updates.dismissed = true)}
    >
      <Icon name="x" size={16} />
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 8px 18px;
    border-bottom: 1px solid var(--line);
    background: var(--bg-selected);
    color: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-weight: 600;
  }
</style>
