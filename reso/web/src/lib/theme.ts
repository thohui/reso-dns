import { createSystem, defaultConfig, defineConfig } from '@chakra-ui/react';

const config = defineConfig({
	globalCss: {
		'html, body': {
			bg: 'bg',
			color: 'fg',
			lineHeight: '1.6',
		},
	},
	theme: {
		semanticTokens: {
			colors: {
				bg: {
					DEFAULT: { value: '#0a0a14' },
					panel: { value: '#10101c' },
					subtle: { value: '#191928' },
					input: { value: '#13131f' },
					elevated: { value: '#1c1c2e' },
				},
				fg: {
					DEFAULT: { value: '#f0f0f4' },
					muted: { value: '#a0a0b4' },
					subtle: { value: '#6a6a82' },
					faint: { value: '#4a4a62' },
				},
				border: {
					DEFAULT: { value: '#1c1c2e' },
					input: { value: '#2a2a40' },
					accent: { value: '#e91e78' },
				},
				accent: {
					DEFAULT: { value: '#e91e78' },
					hover: { value: '#d41a6c' },
					fg: { value: '#f472a8' },
					bg: { value: 'rgba(233, 30, 120, 0.2)' },
					subtle: { value: '#ec4899' },
					muted: { value: 'rgba(233, 30, 120, 0.12)' },
				},
				status: {
					success: { value: '#34d399' },
					error: { value: '#f87171' },
					blocked: { value: '#fb923c' },
					cached: { value: '#60a5fa' },
					info: { value: '#60a5fa' },
					warn: { value: '#fbbf24' },
					successMuted: { value: 'rgba(52,211,153,0.12)' },
					errorMuted: { value: 'rgba(248,113,113,0.12)' },
					blockedMuted: { value: 'rgba(251,146,60,0.12)' },
					cachedMuted: { value: 'rgba(96,165,250,0.12)' },
					warnMuted: { value: 'rgba(250,204,21,0.12)' },
				},
				neutral: {
					fg: { value: '#94a3b8' },
					muted: { value: 'rgba(255,255,255,0.06)' },
				},
			},
		},
	},
});

export const system = createSystem(defaultConfig, config);

export function hexToRgba(hex: string, alpha: number) {
	const h = hex.replace('#', '');
	const full =
		h.length === 3
			? h
					.split('')
					.map((c) => c + c)
					.join('')
			: h;
	const r = parseInt(full.slice(0, 2), 16);
	const g = parseInt(full.slice(2, 4), 16);
	const b = parseInt(full.slice(4, 6), 16);
	return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}
