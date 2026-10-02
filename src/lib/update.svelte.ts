// Windows self-update. Looks for a new release after unlocking and every 12
// hours while Reach runs, when Settings allows it. Linux gets updates from Flathub.
import { api, type UpdateInfo } from './api';

const RECHECK_MS = 12 * 60 * 60 * 1000;

export const updates = $state({
  /** False on Linux. */
  supported: false,
  available: null as UpdateInfo | null,
  /** Banner closed with "Later"; it comes back for a newer version or next start. */
  dismissed: false,
  checking: false,
  installing: false,
});

const supported = api.updateSupported().then(
  (s) => (updates.supported = s),
  () => false,
);
let lastCheck = 0;

/** Ask GitHub now. Throws if the check fails. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  updates.checking = true;
  try {
    const info = await api.updateCheck();
    lastCheck = Date.now();
    if (info?.version !== updates.available?.version) updates.dismissed = false;
    updates.available = info;
    return info;
  } finally {
    updates.checking = false;
  }
}

/** The automatic check: at most every 12 hours, and silent when offline. */
export async function autoCheck() {
  if (!(await supported) || updates.checking || Date.now() - lastCheck < RECHECK_MS) return;
  await checkForUpdate().catch(() => {});
}

/** Download and install. On success Reach exits and the installer restarts it. */
export async function installUpdate() {
  updates.installing = true;
  try {
    await api.updateInstall();
  } finally {
    updates.installing = false;
  }
}
