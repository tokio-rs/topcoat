import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    emptyOutDir: true,
    outDir: '../assets',
    cssCodeSplit: false,
    rollupOptions: {
      input: 'src/main.jsx',
      output: {
        entryFileNames: 'app.js',
        assetFileNames: 'app.css',
      },
    },
  },
})
