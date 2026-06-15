import react from '@vitejs/plugin-react';
import { readFileSync } from 'fs';
import { defineConfig } from 'vite';
import svgr from 'vite-plugin-svgr';

const { version } = JSON.parse(readFileSync('./package.json', 'utf-8')) as {
	version: string;
};

// https://vite.dev/config/
export default defineConfig({
	define: {
		__RESO_VERSION__: JSON.stringify(version),
	},
	resolve: {
		alias: {
			'@': '/src',
		},
	},
	plugins: [
		react(),
		svgr({
			include: '**/*.svg?react',
			svgrOptions: {
				// plugins: ['@svgr/plugin-svgo']
			},
		}),
	],
});
