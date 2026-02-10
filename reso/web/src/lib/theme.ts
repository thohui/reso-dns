import { createSystem, defaultConfig, defineConfig } from '@chakra-ui/react';

const config = defineConfig({
	globalCss: {
		'html, body': {
			bg: 'bg',
			color: 'fg',
		},
	},
	theme: {
		semanticTokens: {
			colors: {
				bg: {
					DEFAULT: { value: '{colors.gray.950}' },
					panel: { value: '{colors.gray.900}' },
					subtle: { value: '{colors.gray.800}' },
					input: { value: '{colors.gray.800}' },
				},
				fg: {
					DEFAULT: { value: '{colors.white}' },
					muted: { value: '{colors.gray.400}' },
					subtle: { value: '{colors.gray.500}' },
					faint: { value: '{colors.gray.600}' },
				},
				border: {
					DEFAULT: { value: '{colors.gray.800}' },
					input: { value: '{colors.gray.700}' },
				},
				accent: {
					DEFAULT: { value: '{colors.green.600}' },
					hover: { value: '{colors.green.700}' },
					fg: { value: '{colors.green.400}' },
					subtle: { value: '{colors.green.500}' },
				},
				// Status
				status: {
					success: { value: '{colors.green.400}' },
					error: { value: '{colors.red.400}' },
					blocked: { value: '{colors.red.400}' },
				},
			},
		},
	},
});

export const system = createSystem(defaultConfig, config);
