import adapter from '@sveltejs/adapter-static';
//import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/kit/vite';
import path from "path";

//const build_in = process.env.NODE_ENV;

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess({
		postcss: true,
		replace: [
			['comptime.port', '3000'],
			['comptime.https_port', '443']
		]
	}),

	kit: {
		adapter: adapter({
			fallback: 'index.html'
		}),
		// paths:  {
		// 	base: build_in === 'production' ? '/analyze' : ''
		// },
		alias: {
			$components: path.resolve("./src/lib/components")
		},
		prerender: { entries: []}
	}
};

export default config;
