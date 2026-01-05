import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    outDir: 'dist/js',
    emptyOutDir: false,
    lib: {
      entry: resolve(__dirname, 'js/editor.js'),
      name: 'GMEditor',
      fileName: 'editor',
      formats: ['es'],
    },
    rollupOptions: {
      output: {
        // Keep as single file
        inlineDynamicImports: true,
      },
    },
  },
});
