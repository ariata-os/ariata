import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		proxy: {
			'/api': {
				target: 'http://localhost:8000',
				changeOrigin: true,
				bypass: (req) => {
					// Keep app-specific endpoints in SvelteKit (use app database)
					if (req.url?.startsWith('/api/preferences')) return req.url;
					if (req.url?.startsWith('/api/dashboards')) return req.url;
					// Everything else goes to Rust backend (ELT database)
					return null;
				}
			}
		}
	}
});
