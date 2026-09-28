// Helpers for turning the flat folder/connection lists into a tree.
import type { Connection, Folder } from './api';

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });
export const byName = (a: { name: string }, b: { name: string }) => collator.compare(a.name, b.name);

/** "Parent / Child" path for a folder. */
export function folderPath(folders: Folder[], id: string | null): string {
  const parts: string[] = [];
  const seen = new Set<string>();
  let cursor = folders.find((f) => f.id === id);
  while (cursor && !seen.has(cursor.id)) {
    seen.add(cursor.id);
    parts.unshift(cursor.name);
    cursor = folders.find((f) => f.id === cursor!.parentId);
  }
  return parts.join(' / ');
}

/** Every folder with its full path, sorted, for <select> lists. */
export function folderOptions(folders: Folder[], exclude?: string) {
  return folders
    .filter((f) => !exclude || !isInside(folders, f.id, exclude))
    .map((f) => ({ id: f.id, label: folderPath(folders, f.id) }))
    .sort((a, b) => collator.compare(a.label, b.label));
}

/** Is `id` the folder `ancestor` or somewhere inside it? */
export function isInside(folders: Folder[], id: string | null, ancestor: string): boolean {
  const seen = new Set<string>();
  let cursor: string | null = id;
  while (cursor && !seen.has(cursor)) {
    if (cursor === ancestor) return true;
    seen.add(cursor);
    cursor = folders.find((f) => f.id === cursor)?.parentId ?? null;
  }
  return false;
}

export type Row =
  | { kind: 'folder'; folder: Folder; depth: number; open: boolean; count: number }
  | { kind: 'connection'; connection: Connection; depth: number; path?: string };

/** Flatten the tree into visible rows, folders first, both sorted by name. */
export function visibleRows(
  folders: Folder[],
  connections: Connection[],
  open: Set<string>,
): Row[] {
  const rows: Row[] = [];
  const known = new Set(folders.map((f) => f.id));
  // A folder whose parent is missing is shown at the top level instead of lost.
  const parentOf = (id: string | null) => (id && known.has(id) ? id : null);
  const countIn = (id: string): number =>
    connections.filter((c) => c.folderId === id).length +
    folders.filter((f) => f.parentId === id).reduce((n, f) => n + countIn(f.id), 0);

  const walk = (parent: string | null, depth: number) => {
    for (const folder of folders.filter((f) => parentOf(f.parentId) === parent).sort(byName)) {
      const isOpen = open.has(folder.id);
      rows.push({ kind: 'folder', folder, depth, open: isOpen, count: countIn(folder.id) });
      if (isOpen) walk(folder.id, depth + 1);
    }
    for (const connection of connections.filter((c) => parentOf(c.folderId) === parent).sort(byName)) {
      rows.push({ kind: 'connection', connection, depth });
    }
  };
  walk(null, 0);
  return rows;
}

/** Search results: matching connections as a flat list with their folder path. */
export function searchRows(folders: Folder[], connections: Connection[], query: string): Row[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return connections
    .filter((c) => {
      const text = `${c.name} ${c.host} ${c.username} ${folderPath(folders, c.folderId)}`.toLowerCase();
      return words.every((w) => text.includes(w));
    })
    .sort(byName)
    .map((connection) => ({
      kind: 'connection' as const,
      connection,
      depth: 0,
      path: folderPath(folders, connection.folderId),
    }));
}
