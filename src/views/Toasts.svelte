<script lang="ts">
  import Icon from '../lib/Icon.svelte';
  import { dismiss, toasts } from '../lib/toast.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each toasts as t (t.id)}
    <div class="toast {t.kind}">
      <span>{t.text}</span>
      <button class="btn ghost icon" onclick={() => dismiss(t.id)} aria-label="Dismiss">
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(380px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 6px 10px 14px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--accent);
    border-radius: var(--radius);
    background: var(--bg-raised);
    box-shadow: var(--shadow);
    animation: rise 0.18s ease-out;
  }
  .toast span {
    flex: 1;
    padding-top: 5px;
    white-space: pre-line;
    user-select: text;
    -webkit-user-select: text;
  }
  .toast.error {
    border-left-color: var(--danger);
  }
  .toast .btn {
    height: 28px;
    width: 28px;
  }
  @keyframes rise {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
