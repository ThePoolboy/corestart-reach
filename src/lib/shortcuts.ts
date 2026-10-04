// Keyboard shortcuts of the main window. Each has a default here; the ones the
// user changes in Settings are kept in the vault's settings (`shortcuts`).
import type { Settings } from './api';

export const SHORTCUTS = [
  { action: 'quickConnect', label: 'Quick connect', keys: 'Ctrl+K' },
  { action: 'newConnection', label: 'New connection', keys: 'Ctrl+N' },
  { action: 'search', label: 'Search', keys: 'Ctrl+F' },
  { action: 'lock', label: 'Lock Reach', keys: 'Ctrl+L' },
  { action: 'settings', label: 'Settings', keys: 'Ctrl+,' },
  { action: 'save', label: 'Save a connection', keys: 'Ctrl+S' },
] as const;

export type ShortcutAction = (typeof SHORTCUTS)[number]['action'];

export function defaultKeys(action: ShortcutAction): string {
  return SHORTCUTS.find((s) => s.action === action)!.keys;
}

/** The keys for an action: the user's own, or the default. */
export function keysFor(settings: Settings, action: ShortcutAction): string {
  return settings.shortcuts?.[action] ?? defaultKeys(action);
}

const MODIFIER_KEYS = ['Control', 'Alt', 'AltGraph', 'Shift', 'Meta', 'OS', 'CapsLock', 'NumLock', 'Dead', 'Unidentified'];

/** "Ctrl+Shift+K" for a key press, or null while only modifiers are held. Cmd
 *  counts as Ctrl, as it always has in Reach. */
export function keysOf(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.includes(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  parts.push(e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key);
  return parts.join('+');
}

/** Is this key press the shortcut for `action`? */
export function pressed(e: KeyboardEvent, settings: Settings, action: ShortcutAction): boolean {
  return keysOf(e) === keysFor(settings, action);
}

/** Keys that text boxes and windows need. */
const RESERVED: Record<string, string> = {
  'Ctrl+C': 'copying',
  'Ctrl+V': 'pasting',
  'Ctrl+X': 'cutting',
  'Ctrl+A': 'selecting all',
  'Ctrl+Z': 'undo',
  'Ctrl+Y': 'redo',
  'Ctrl+Shift+Z': 'redo',
  'Alt+F4': 'closing the window',
};

/** Why `keys` (pressed as `e`) can't be the shortcut for `action`, or null if it can. */
export function problemWith(e: KeyboardEvent, keys: string, action: ShortcutAction, settings: Settings): string | null {
  const functionKey = /^F([1-9]|1[0-2])$/.test(e.key);
  if (!(e.ctrlKey || e.metaKey || e.altKey) && !functionKey) {
    return 'Use Ctrl or Alt with a key, or a function key (F1 to F12), so typing still works.';
  }
  if (RESERVED[keys]) return `${keys} is used for ${RESERVED[keys]}.`;
  const other = SHORTCUTS.find((s) => s.action !== action && keysFor(settings, s.action) === keys);
  if (other) return `${keys} is already the shortcut for ${other.label}.`;
  return null;
}
