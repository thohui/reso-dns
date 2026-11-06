export const normalizeHostname = (hostname?: string | null) =>
	hostname?.split('.')[0];

export function formatClient(ip: string | null, name?: string | null): string {
	if (!name) return ip ?? 'unknown';
	return `${name} (${ip})`;
}
