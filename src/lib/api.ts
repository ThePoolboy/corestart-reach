// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
import { Channel, invoke } from '@tauri-apps/api/core';

export type Protocol = 'rdp' | 'ssh';
export type RdpScreen = 'window' | 'fullscreen';

export interface Folder {
  id: string;
  name: string;
  parentId: string | null;
}

export interface Connection {
  id: string;
  name: string;
  folderId: string | null;
  protocol: Protocol;
  host: string;
  port: number | null;
  credentialId: string | null;
  username: string;
  domain: string;
  hasPassword: boolean;
  sshKeyPath: string;
  hasKeyPassphrase: boolean;
  rdpScreen: RdpScreen;
  notes: string;
}

export interface Credential {
  id: string;
  name: string;
  username: string;
  domain: string;
  hasPassword: boolean;
}

export interface Tree {
  folders: Folder[];
  connections: Connection[];
  credentials: Credential[];
}

export interface Saved {
  id: string;
  tree: Tree;
}

/** Keep the stored secret, remove it, or replace it. */
export type SecretUpdate = 'keep' | 'clear' | { set: string };

export interface ConnectionInput {
  id: string | null;
  name: string;
  folderId: string | null;
  protocol: Protocol;
  host: string;
  port: number | null;
  credentialId: string | null;
  username: string;
  domain: string;
  password: SecretUpdate;
  sshKeyPath: string;
  sshKeyPassphrase: SecretUpdate;
  rdpScreen: RdpScreen;
  notes: string;
}

export interface CredentialInput {
  id: string | null;
  name: string;
  username: string;
  domain: string;
  password: SecretUpdate;
}

export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
  path: string;
  minPasswordLength: number;
}

/** A login typed at connect time. Used once, never saved. */
export interface Login {
  username: string;
  domain: string;
  password: string;
}

export type ConnectOutcome =
  | { status: 'launched' }
  | { status: 'needCredentials'; username: string; domain: string };

export type SshEvent =
  | { type: 'status'; message: string }
  | { type: 'hostKey'; host: string; port: number; algorithm: string; fingerprint: string }
  | { type: 'prompt'; prompt: string; secret: boolean }
  | { type: 'connected' }
  | { type: 'closed'; message: string; exited: boolean }
  | { type: 'failed'; message: string };

export type SshAnswer =
  | { type: 'hostKey'; accept: boolean }
  | { type: 'prompt'; value: string | null };

export const api = {
  vaultStatus: () => invoke<VaultStatus>('vault_status'),
  vaultCreate: (password: string) => invoke<Tree>('vault_create', { password }),
  vaultUnlock: (password: string) => invoke<Tree>('vault_unlock', { password }),
  vaultLock: () => invoke<void>('vault_lock'),
  getTree: () => invoke<Tree>('get_tree'),

  saveConnection: (input: ConnectionInput) => invoke<Saved>('save_connection', { input }),
  duplicateConnection: (id: string) => invoke<Saved>('duplicate_connection', { id }),
  deleteConnection: (id: string) => invoke<Tree>('delete_connection', { id }),
  saveFolder: (input: { id: string | null; name: string; parentId: string | null }) =>
    invoke<Saved>('save_folder', { input }),
  deleteFolder: (id: string) => invoke<Tree>('delete_folder', { id }),
  /** Move a connection or folder into `folderId` (null = top level). */
  moveItem: (kind: 'connection' | 'folder', id: string, folderId: string | null) =>
    invoke<Tree>('move_item', { kind, id, folderId }),
  saveCredential: (input: CredentialInput) => invoke<Saved>('save_credential', { input }),
  deleteCredential: (id: string) => invoke<Tree>('delete_credential', { id }),

  connect: (id: string, login?: Login) => invoke<ConnectOutcome>('connect', { id, login: login ?? null }),
  quickConnect: (protocol: Protocol, address: string, login?: Login) =>
    invoke<ConnectOutcome>('quick_connect', { protocol, address, login: login ?? null }),

  sshTitle: (session: string) => invoke<string>('ssh_title', { session }),
  sshStart: (
    session: string,
    cols: number,
    rows: number,
    output: Channel<ArrayBuffer | number[]>,
    events: Channel<SshEvent>,
  ) => invoke<void>('ssh_start', { session, cols, rows, output, events }),
  sshWrite: (session: string, data: string) => invoke<void>('ssh_write', { session, data }),
  sshResize: (session: string, cols: number, rows: number) =>
    invoke<void>('ssh_resize', { session, cols, rows }),
  sshAnswer: (session: string, answer: SshAnswer) =>
    invoke<void>('ssh_answer', { session, answer }),
};

/** Errors from Rust arrive as plain strings. */
export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
