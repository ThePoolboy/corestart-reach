// Backups take, besides the encrypted data, the choices this window keeps in
// local storage: theme, open folders, the Quick connect protocol. Every key
// starts with "reach.", so new ones come along without changes here.
import type { BackupSummary, UiState } from './api';
import { reloadTheme } from './theme.svelte';

const PREFIX = 'reach.';

export const BACKUP_FILTERS = [{ name: 'Reach backup', extensions: ['reachbackup'] }];

/** "Reach backup 2026-10-05.reachbackup", in local time. */
export function backupFileName(): string {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, '0');
  return `Reach backup ${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}.reachbackup`;
}

/** The last part of a path, on Windows or Linux. */
export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function rememberedChoices(): UiState {
  const ui: UiState = {};
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      const value = key?.startsWith(PREFIX) ? localStorage.getItem(key) : null;
      if (key && value !== null) ui[key] = value;
    }
  } catch {
    // Storage unavailable: the backup just has none.
  }
  return ui;
}

/** Swap this window's remembered choices for the backup's. */
export function restoreRememberedChoices(ui: UiState) {
  try {
    const old: string[] = [];
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(PREFIX)) old.push(key);
    }
    for (const key of old) localStorage.removeItem(key);
    for (const [key, value] of Object.entries(ui)) {
      if (key.startsWith(PREFIX)) localStorage.setItem(key, value);
    }
  } catch {
    // Storage unavailable: the connections are restored all the same.
  }
  reloadTheme();
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** "3 folders, 24 connections and 5 saved credentials". */
export function describeBackup(s: BackupSummary): string {
  return `${plural(s.folders, 'folder', 'folders')}, ${plural(s.connections, 'connection', 'connections')} and ${plural(s.credentials, 'saved credential', 'saved credentials')}`;
}

/** "5 October 2026" in the system's language. */
export function backupDate(s: BackupSummary): string {
  return new Date(s.created * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'long', year: 'numeric' });
}
