<script lang="ts">
  import { untrack } from 'svelte';
  import type { Protocol } from '../lib/api';
  import Modal from './Modal.svelte';

  // Connect to a host without saving it. RDP then asks for a login in the
  // "Log in" dialog; SSH asks inside its terminal window.
  let {
    initial = '',
    onsubmit,
    oncancel,
  }: {
    initial?: string;
    onsubmit: (protocol: Protocol, address: string) => void;
    oncancel: () => void;
  } = $props();

  const PROTOCOL_KEY = 'reach.quickProtocol';

  function lastProtocol(): Protocol {
    try {
      return localStorage.getItem(PROTOCOL_KEY) === 'ssh' ? 'ssh' : 'rdp';
    } catch {
      return 'rdp';
    }
  }

  let protocol = $state<Protocol>(lastProtocol());
  let address = $state(untrack(() => initial));

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!address.trim()) return;
    try {
      localStorage.setItem(PROTOCOL_KEY, protocol);
    } catch {
      // only a convenience
    }
    onsubmit(protocol, address.trim());
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<Modal title="Quick connect" {oncancel}>
  <form onsubmit={submit} class="form">
    <div class="segmented">
      <button type="button" class:on={protocol === 'rdp'} onclick={() => (protocol = 'rdp')}>RDP</button>
      <button type="button" class:on={protocol === 'ssh'} onclick={() => (protocol = 'ssh')}>SSH</button>
    </div>
    <label class="field">
      <span>Host</span>
      <input
        class="input"
        bind:value={address}
        placeholder={protocol === 'ssh' ? 'server, 10.0.0.5, user@host:2222' : 'server, 10.0.0.5, pc01.corp.example.com'}
        spellcheck="false"
        autocomplete="off"
        use:focus
      />
      <span class="hint">
        Host name, FQDN or IP, with an optional <code>:port</code>.
        {protocol === 'ssh' ? 'You log in inside the terminal window.' : "You'll be asked for a login next."}
        Nothing is saved.
      </span>
    </label>
    <div class="actions">
      <button type="button" class="btn" onclick={oncancel}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!address.trim()}>Connect</button>
    </div>
  </form>
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .segmented {
    align-self: flex-start;
  }
  code {
    font-family: var(--mono);
    font-size: 11.5px;
  }
</style>
