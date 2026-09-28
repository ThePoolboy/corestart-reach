<script lang="ts">
  import type { SecretUpdate } from '../lib/api';

  // A password box for a secret the UI never gets to see. If one is saved it
  // shows "Saved" with Change/Remove; the result is a SecretUpdate for Rust.
  let {
    label,
    saved,
    value = $bindable(),
    placeholder = '',
    hint = '',
  }: {
    label: string;
    saved: boolean;
    value: SecretUpdate;
    placeholder?: string;
    hint?: string;
  } = $props();

  let editing = $state(false);
  let text = $state('');
  let reveal = $state(false);

  // The parent remounts this component (via {#key}) when it loads another item
  // or after saving, so local state never needs resetting here.

  function input() {
    value = text ? { set: text } : 'keep';
  }

  function change() {
    editing = true;
    text = '';
  }

  function cancel() {
    editing = false;
    text = '';
    value = 'keep';
  }

  function focus(node: HTMLInputElement) {
    node.focus();
  }
</script>

<div class="field">
  <span>{label}</span>
  {#if saved && value === 'clear'}
    <div class="state">
      <span class="pill removed">Will be removed when you save</span>
      <button type="button" class="btn ghost" onclick={cancel}>Undo</button>
    </div>
  {:else if saved && !editing}
    <div class="state">
      <span class="pill">•••••••• Saved</span>
      <button type="button" class="btn ghost" onclick={change}>Change</button>
      <button type="button" class="btn ghost danger" onclick={() => (value = 'clear')}>Remove</button>
    </div>
  {:else}
    <div class="state">
      {#if editing}
        <input
          class="input"
          type={reveal ? 'text' : 'password'}
          bind:value={text}
          oninput={input}
          {placeholder}
          autocomplete="new-password"
          use:focus
        />
      {:else}
        <input
          class="input"
          type={reveal ? 'text' : 'password'}
          bind:value={text}
          oninput={input}
          {placeholder}
          autocomplete="new-password"
        />
      {/if}
      <button type="button" class="btn ghost" onclick={() => (reveal = !reveal)}>
        {reveal ? 'Hide' : 'Show'}
      </button>
      {#if saved}
        <button type="button" class="btn ghost" onclick={cancel}>Cancel</button>
      {/if}
    </div>
  {/if}
  {#if hint}
    <span class="hint">{hint}</span>
  {/if}
</div>

<style>
  .state {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .state .input {
    flex: 1;
  }
  .pill {
    flex: 1;
    display: flex;
    align-items: center;
    height: 34px;
    padding: 0 10px;
    border: 1px dashed var(--line-strong);
    border-radius: var(--radius-sm);
    color: var(--text-dim);
  }
  .pill.removed {
    color: var(--danger);
    border-color: var(--danger);
  }
</style>
