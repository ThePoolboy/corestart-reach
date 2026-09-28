// Make the webview behave like a desktop app rather than a web page.

/** No browser menu (Back / Reload / Inspect). Text fields and selected text keep
 *  theirs for copy and paste; the connection tree has its own menu. */
export function blockBrowserMenu() {
  window.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement | null;
    if (target?.closest('input, textarea, [contenteditable="true"]')) return;
    if (window.getSelection()?.toString()) return;
    e.preventDefault();
  });
}

/** No browser shortcuts in the main window of a release build: reload (F5,
 *  Ctrl+R), print, find, view source, downloads, history. WebView2 on Windows
 *  acts on these unless the page stops them. Not used in SSH windows: the
 *  terminal already swallows every key it sends to the server (Ctrl+R is
 *  bash's history search). */
export function blockBrowserShortcuts() {
  if (import.meta.env.DEV) return; // keep reload handy while developing
  window.addEventListener('keydown', (e) => {
    const key = e.key.toLowerCase();
    const mod = e.ctrlKey || e.metaKey;
    if (key === 'f5' || key === 'f7' || key === 'browserrefresh') {
      e.preventDefault();
    } else if (mod && ['r', 'p', 'f', 'g', 'u', 'j', 'h', 's', 'o'].includes(key)) {
      // The app's own Ctrl+F still runs: preventDefault doesn't stop other listeners.
      e.preventDefault();
    }
  });
}
