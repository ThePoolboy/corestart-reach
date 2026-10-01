import { getCurrentWindow } from '@tauri-apps/api/window';

// Light or dark look for the main window. Kept in the webview's local storage,
// not the vault, because the lock screen needs it before the vault is open.
// SSH windows stay dark, like most terminals.

export type ThemeChoice = 'system' | 'light' | 'dark';

const KEY = 'reach.theme';
const systemLight = window.matchMedia('(prefers-color-scheme: light)');

function load(): ThemeChoice {
  try {
    const saved = localStorage.getItem(KEY);
    if (saved === 'light' || saved === 'dark') return saved;
  } catch {
    // Storage unavailable: follow the system.
  }
  return 'system';
}

export const theme = $state({ choice: load(), light: false });

function apply() {
  theme.light = theme.choice === 'system' ? systemLight.matches : theme.choice === 'light';
  document.documentElement.dataset.theme = theme.light ? 'light' : 'dark';
}

/** Apply the saved choice and follow system changes. Call once, before mounting. */
export function initTheme() {
  apply();
  // Title bar (Windows) and native widgets match the chosen look.
  if (theme.choice !== 'system') getCurrentWindow().setTheme(theme.choice).catch(() => {});
  systemLight.addEventListener('change', () => {
    if (theme.choice === 'system') apply();
  });
}

export function setTheme(choice: ThemeChoice) {
  theme.choice = choice;
  try {
    if (choice === 'system') localStorage.removeItem(KEY);
    else localStorage.setItem(KEY, choice);
  } catch {
    // Not saved; it still applies until Reach closes.
  }
  // With the window forced to one theme the media query reports that theme,
  // so release it first and read the system's real preference afterwards.
  getCurrentWindow()
    .setTheme(choice === 'system' ? null : choice)
    .catch(() => {})
    .finally(apply);
  apply();
}

/** The quick toggle: flip whatever is showing now. */
export function toggleTheme() {
  setTheme(theme.light ? 'dark' : 'light');
}
