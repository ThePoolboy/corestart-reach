import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri expects a fixed dev port and handles the screen itself.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Flatpak build folders contain a whole filesystem, with symlink loops.
    watch: { ignored: ['**/src-tauri/**', '**/build-flatpak/**', '**/.flatpak-builder/**'] },
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: true,
  },
});
