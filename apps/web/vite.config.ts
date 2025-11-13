import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, loadEnv } from 'vite';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';

// Get current directory in ES modules
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Load .env from project root into process.env BEFORE SvelteKit initializes
dotenv.config({ path: path.resolve(__dirname, '../../.env') });

export default defineConfig(({ mode }) => {
	// Load env file from project root (../..)
	const env = loadEnv(mode, '../..', '');

	return {
		envDir: '../..', // Load .env from project root
		plugins: [tailwindcss(), sveltekit()],
		server: {
			proxy: {
				'/api': {
					target: env.ELT_API_URL || 'http://localhost:8000',
					changeOrigin: true,
					bypass: (req) => {
						// Keep app-specific endpoints in SvelteKit (use app database)
						if (req.url?.startsWith('/api/app')) return req.url;
						// Keep chat endpoint in SvelteKit
						if (req.url?.startsWith('/api/chat')) return req.url;
						// Keep sessions endpoint in SvelteKit (chat session management)
						if (req.url?.startsWith('/api/sessions')) return req.url;
						// Keep preferences endpoint in SvelteKit (user preferences)
						if (req.url?.startsWith('/api/preferences')) return req.url;
						// Everything else goes to Rust backend (ELT database)
						return null;
					}
				}
			}
		}
	};
});
