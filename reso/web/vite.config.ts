import { readFileSync } from 'node:fs';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';
import svgr from 'vite-plugin-svgr';

const cargoTomlPath = new URL('../Cargo.toml', import.meta.url);
const cargoToml = readFileSync(cargoTomlPath, 'utf-8');

const version = /^\[package\][^[]*?^version\s*=\s*"([^"]+)"/ms.exec(
	cargoToml,
)?.[1];
if (!version) {
	throw new Error('failed to read version from ../Cargo.toml');
}

// https://vite.dev/config
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
			svgrOptions: {},
		}),
	],
});
