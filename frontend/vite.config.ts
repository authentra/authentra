import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import UnoCSS from 'unocss/vite'
import type { Theme } from 'unocss/preset-uno'
import unoconfig from './unocss.config'


// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    UnoCSS<Theme>(unoconfig),
    svelte()
  ],
})
