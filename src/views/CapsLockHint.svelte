<script lang="ts" module>
  // Browsers only say whether Caps Lock is on as part of a key or mouse event, so
  // every hint shares what the last one said, and which element has focus.
  const keyboard = $state({ capsLock: false, focus: null as Element | null });
  let watching = false;

  function watch() {
    if (watching) return;
    watching = true;
    // On Linux the Caps Lock key's own keydown can still report the old state;
    // its keyup has the new one.
    const read = (e: KeyboardEvent | MouseEvent) => (keyboard.capsLock = e.getModifierState('CapsLock'));
    for (const type of ['keydown', 'keyup', 'mousedown'] as const) window.addEventListener(type, read, true);
    window.addEventListener('focusin', () => (keyboard.focus = document.activeElement));
    window.addEventListener('focusout', () => (keyboard.focus = null));
    // Caps Lock may change in another window: wait for the next key or click.
    window.addEventListener('blur', () => (keyboard.capsLock = false));
    keyboard.focus = document.activeElement;
  }
</script>

<script lang="ts">
  // "Caps Lock is on." while you type in a password box with Caps Lock on. It
  // covers the password boxes inside the element it's placed in.
  watch();
  let hint = $state<HTMLElement>();
  const show = $derived(
    keyboard.capsLock &&
      keyboard.focus instanceof HTMLInputElement &&
      keyboard.focus.type === 'password' &&
      !!hint?.parentElement?.contains(keyboard.focus),
  );
</script>

<span class="caps" role="status" bind:this={hint}>{#if show}Caps Lock is on.{/if}</span>

<style>
  .caps {
    font-size: 12px;
    font-weight: 400;
    color: var(--danger);
  }
  .caps:empty {
    display: none;
  }
</style>
