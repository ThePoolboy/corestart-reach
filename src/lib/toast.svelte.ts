// Small notification queue shown in the bottom corner of the main window.
import { errorText } from './api';

export type ToastKind = 'info' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

let nextId = 1;

export const toasts = $state<Toast[]>([]);

export function toast(text: string, kind: ToastKind = 'info') {
  const id = nextId++;
  toasts.push({ id, kind, text });
  setTimeout(() => dismiss(id), kind === 'error' ? 8000 : 3500);
}

export function toastError(e: unknown) {
  toast(errorText(e), 'error');
}

export function dismiss(id: number) {
  const i = toasts.findIndex((t) => t.id === id);
  if (i >= 0) toasts.splice(i, 1);
}
