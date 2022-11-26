import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        signup: resolve(__dirname, "signup.html"),
        editor: resolve(__dirname, "editor.html"),
        validate: resolve(__dirname, "validate.html"),
      }
    }
  },
  define: {
    __APP_VERSION__: JSON.stringify(require('./package.json').version),
  }
})
