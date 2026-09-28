<script lang="ts" module>
  import type { IconName } from '../lib/Icon.svelte';

  export type MenuItem =
    | { label: string; icon?: IconName; danger?: boolean; action: () => void }
    | 'separator';
</script>

<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import Icon from '../lib/Icon.svelte';

  // Right-click menu at the pointer. Closes on Escape, an outside click,
  // or when the window loses focus.
  let { x, y, items, onclose }: { x: number; y: number; items: MenuItem[]; onclose: () => void } =
    $props();

  let menu: HTMLDivElement;
  let pos = $state(untrack(() => ({ left: x, top: y })));

  onMount(() => {
    // Keep the whole menu on screen.
    const r = menu.getBoundingClientRect();
    pos = {
      left: Math.max(4, Math.min(pos.left, window.innerWidth - r.width - 4)),
      top: Math.max(4, Math.min(pos.top, window.innerHeight - r.height - 4)),
    };
    menu.querySelector('button')?.focus();
  });

  function pick(item: Exclude<MenuItem, 'separator'>) {
    onclose();
    item.action();
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const buttons = [...menu.querySelectorAll('button')];
      const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = (i + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }
  }

  function outside(e: MouseEvent) {
    if (!menu.contains(e.target as Node)) onclose();
  }
</script>

<svelte:window onmousedown={outside} onblur={onclose} onresize={onclose} />

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="ctx"
  role="menu"
  tabindex="-1"
  bind:this={menu}
  style="left: {pos.left}px; top: {pos.top}px"
  onkeydown={keydown}
  oncontextmenu={(e) => e.preventDefault()}
>
  {#each items as item, i (i)}
    {#if item === 'separator'}
      <div class="sep" role="separator"></div>
    {:else}
      <button role="menuitem" class:danger={item.danger} onclick={() => pick(item)}>
        {#if item.icon}<Icon name={item.icon} />{:else}<span class="gap"></span>{/if}
        {item.label}
      </button>
    {/if}
  {/each}
</div>

<style>
  .ctx {
    position: fixed;
    z-index: 60;
    min-width: 200px;
    padding: 5px;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    background: var(--bg-raised);
    box-shadow: var(--shadow);
    outline: none;
  }
  button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    outline: none;
  }
  button:hover,
  button:focus-visible {
    background: var(--bg-hover);
  }
  button.danger {
    color: var(--danger);
  }
  .gap {
    width: 16px;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
</style>
