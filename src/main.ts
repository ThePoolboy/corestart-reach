import { mount } from 'svelte';
import './app.css';
import App from './views/App.svelte';
import SshSession from './views/SshSession.svelte';

// One bundle, two kinds of window: the main manager and SSH session windows
// (opened by Rust as index.html?view=ssh&session=<id>).
const params = new URLSearchParams(location.search);
const target = document.getElementById('app')!;
const session = params.get('session');

if (params.get('view') === 'ssh' && session) {
  mount(SshSession, { target, props: { session } });
} else {
  mount(App, { target });
}
