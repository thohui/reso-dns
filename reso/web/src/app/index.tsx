import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './app.tsx';

// biome-ignore lint/style/noNonNullAssertion: this should always exist.
const root = document.getElementById('root')!;

createRoot(root).render(
	<StrictMode>
		<App />
	</StrictMode>,
);
