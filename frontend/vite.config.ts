import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import UnoCSS from '@unocss/svelte-scoped/vite'
import Icons from 'unplugin-icons/vite';

export default defineConfig({
    server: {
        host: true,
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:8080',
                changeOrigin: true,
            },
        }
    },
	plugins: [
        Icons({
            compiler: 'svelte',
        }),
		UnoCSS({}),
		sveltekit()
	]
});
