import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { SvelteKitPWA} from "@vite-pwa/sveltekit";


export default defineConfig({
	plugins: [sveltekit(),
	SvelteKitPWA({

		manifest: {
			short_name: 'cam-app',
			name: 'Camera App',
			start_url: '/',
			scope: '/',
			display: 'minimal-ui',
			display_override: ['window-controls-overlay'],
			theme_color: "#ffffff",
			background_color: "#ffffff",
			icons: [
				{
					src: '/pwa-192x192.png',
					sizes: '192x192',
					type: 'image/png',
					purpose: 'any maskable',
				}

			],
		},
	})]
});
