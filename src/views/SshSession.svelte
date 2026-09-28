<script lang="ts">
  import { Channel } from '@tauri-apps/api/core';
  import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { FitAddon } from '@xterm/addon-fit';
  import { Terminal } from '@xterm/xterm';
  import '@xterm/xterm/css/xterm.css';
  import { onMount } from 'svelte';
  import { api, errorText, type SshAnswer, type SshEvent } from '../lib/api';

  // One SSH session in its own window. Login questions (host key, password,
  // 2FA) are asked inside the terminal, the same way OpenSSH does.
  let { session }: { session: string } = $props();

  type Mode = 'connecting' | 'prompt' | 'shell' | 'closed';
  let mode = $state<Mode>('connecting');
  let status = $state('Connecting…');
  let container: HTMLDivElement;

  const DIM = '\x1b[90m';
  const RED = '\x1b[91m';
  const RESET = '\x1b[0m';

  const term = new Terminal({
    cursorBlink: true,
    fontFamily: getComputedStyle(document.documentElement).getPropertyValue('--mono').trim() || 'monospace',
    fontSize: 14,
    scrollback: 10000,
    theme: {
      background: '#141415',
      foreground: '#e4e4e4',
      cursor: '#23a0f3',
      cursorAccent: '#141415',
      selectionBackground: 'rgba(35, 160, 243, 0.35)',
      black: '#1d1f21',
      red: '#e5534b',
      green: '#57ab5a',
      yellow: '#c69026',
      blue: '#42a5e9',
      magenta: '#b083f0',
      cyan: '#39c5cf',
      white: '#d0d0d0',
      brightBlack: '#6e7681',
      brightRed: '#ff7b72',
      brightGreen: '#7ee787',
      brightYellow: '#e3b341',
      brightBlue: '#79c0ff',
      brightMagenta: '#d2a8ff',
      brightCyan: '#56d4dd',
      brightWhite: '#ffffff',
    },
  });
  const fit = new FitAddon();
  term.loadAddon(fit);

  // What an in-terminal prompt is waiting for, and what has been typed so far.
  let pending: { kind: 'hostKey' } | { kind: 'text'; secret: boolean } | null = null;
  let typed = '';

  /** Terminal text needs \r\n line endings. */
  const lines = (text: string) => text.replace(/\r?\n/g, '\r\n');

  function answer(a: SshAnswer) {
    pending = null;
    typed = '';
    mode = 'connecting';
    api.sshAnswer(session, a).catch(() => {});
  }

  function handle(event: SshEvent) {
    switch (event.type) {
      case 'status':
        term.write(`${DIM}${lines(event.message)}${RESET}\r\n`);
        break;
      case 'hostKey': {
        const where = event.port === 22 ? event.host : `[${event.host}]:${event.port}`;
        term.write(
          `The authenticity of host '${where}' can't be established.\r\n` +
            `${event.algorithm} key fingerprint is ${event.fingerprint}.\r\n` +
            `Are you sure you want to continue connecting (yes/no)? `,
        );
        pending = { kind: 'hostKey' };
        typed = '';
        mode = 'prompt';
        status = 'Waiting for you to confirm the host key';
        break;
      }
      case 'prompt':
        term.write(lines(event.prompt.endsWith(' ') ? event.prompt : `${event.prompt} `));
        pending = { kind: 'text', secret: event.secret };
        typed = '';
        mode = 'prompt';
        status = 'Waiting for login';
        break;
      case 'connected':
        mode = 'shell';
        status = 'Connected';
        fit.fit();
        api.sshResize(session, term.cols, term.rows).catch(() => {});
        term.focus();
        break;
      case 'closed':
        term.write(`\r\n${DIM}${event.message} Press Enter to reconnect.${RESET}\r\n`);
        mode = 'closed';
        status = 'Disconnected';
        break;
      case 'failed':
        term.write(`\r\n${RED}${lines(event.message)}${RESET}\r\n${DIM}Press Enter to try again.${RESET}\r\n`);
        mode = 'closed';
        status = 'Connection failed';
        break;
    }
  }

  /** Local line editing while a login prompt is showing. */
  function promptInput(data: string) {
    if (!pending) return;
    const secret = pending.kind === 'text' && pending.secret;
    for (const ch of Array.from(data)) {
      if (ch === '\r' || ch === '\n') {
        term.write('\r\n');
        if (pending.kind === 'hostKey') {
          const reply = typed.trim().toLowerCase();
          if (reply === 'yes' || reply === 'no') {
            answer({ type: 'hostKey', accept: reply === 'yes' });
          } else {
            typed = '';
            term.write(`Please type 'yes' or 'no': `);
          }
        } else {
          answer({ type: 'prompt', value: typed });
        }
        return;
      } else if (ch === '\x03') {
        term.write('^C\r\n');
        answer(pending.kind === 'hostKey' ? { type: 'hostKey', accept: false } : { type: 'prompt', value: null });
        return;
      } else if (ch === '\x7f' || ch === '\b') {
        if (typed) {
          typed = Array.from(typed).slice(0, -1).join('');
          if (!secret) term.write('\b \b');
        }
      } else if (ch >= ' ') {
        typed += ch;
        if (!secret) term.write(ch);
      }
    }
  }

  async function start() {
    mode = 'connecting';
    status = 'Connecting…';
    const output = new Channel<ArrayBuffer | number[]>();
    output.onmessage = (data) => term.write(new Uint8Array(data));
    const events = new Channel<SshEvent>();
    events.onmessage = handle;
    try {
      await api.sshStart(session, term.cols, term.rows, output, events);
    } catch (e) {
      handle({ type: 'failed', message: errorText(e) });
    }
  }

  async function copy() {
    const text = term.getSelection();
    if (text) await writeText(text).catch(() => {});
  }

  async function paste() {
    const text = await readText().catch(() => '');
    if (text) term.paste(text);
  }

  onMount(() => {
    term.open(container);
    fit.fit();

    term.onData((data) => {
      if (mode === 'shell') api.sshWrite(session, data).catch(() => {});
      else if (mode === 'prompt') promptInput(data);
      else if (mode === 'closed' && data === '\r') start();
    });
    term.onResize(({ cols, rows }) => {
      if (mode === 'shell') api.sshResize(session, cols, rows).catch(() => {});
    });
    // Ctrl+Shift+C / Ctrl+Shift+V copy and paste, like other Linux terminals.
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== 'keydown' || !e.ctrlKey || !e.shiftKey) return true;
      const key = e.key.toLowerCase();
      if (key === 'c') {
        e.preventDefault();
        copy();
        return false;
      }
      if (key === 'v') {
        e.preventDefault();
        paste();
        return false;
      }
      return true;
    });

    let frame = 0;
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => fit.fit());
    });
    observer.observe(container);

    term.focus();
    start();
    return () => {
      observer.disconnect();
      term.dispose();
    };
  });
</script>

<div class="session">
  <div class="term" bind:this={container}></div>
  <footer>
    <span class="dot {mode}"></span>
    <span class="status">{status}</span>
    <span class="keys">Ctrl+Shift+C copy · Ctrl+Shift+V paste</span>
  </footer>
</div>

<style>
  .session {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #141415;
  }
  .term {
    flex: 1;
    min-height: 0;
    padding: 6px 0 0 8px;
  }
  .term :global(.xterm) {
    height: 100%;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    padding: 0 10px;
    border-top: 1px solid #2a2a2d;
    background: #1b1b1c;
    color: #a2a6ad;
    font-size: 12px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #c69026;
  }
  .dot.shell {
    background: #4caf7d;
  }
  .dot.closed {
    background: #ef5350;
  }
  .status {
    flex: 1;
  }
  .keys {
    color: #72767d;
  }
</style>
