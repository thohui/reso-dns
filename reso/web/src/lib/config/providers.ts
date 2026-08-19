import type { Upstream, UpstreamKind } from '@/lib/api/config';
import { parseHostPort } from '@/lib/config/schema';

export const DEFAULT_PORTS: Record<UpstreamKind, number> = {
	plain: 53,
	tls: 853,
};

export interface ProviderServer {
	address: string;
	label: string;
	kind?: UpstreamKind;
	// Certificate identity, not an address.
	hostname?: string;
}

export interface ProviderGroup {
	name: string;
	description: string;
	servers: ProviderServer[];
}

export const providerGroups: ProviderGroup[] = [
	{
		name: 'Cloudflare',
		description: 'Privacy-first, fast DNS',
		servers: [
			{ address: '1.1.1.1', label: 'Primary' },
			{ address: '1.0.0.1', label: 'Secondary' },
			{
				address: '1.1.1.1',
				label: 'DNS over TLS',
				kind: 'tls',
				hostname: 'cloudflare-dns.com',
			},
			// {
			// 	address: 'https://cloudflare-dns.com/dns-query',
			// 	label: 'DNS over HTTPS',
			// },
		],
	},
	{
		name: 'Google',
		description: 'Reliable, global DNS',
		servers: [
			{ address: '8.8.8.8', label: 'Primary' },
			{ address: '8.8.4.4', label: 'Secondary' },
			{
				address: '8.8.8.8',
				label: 'DNS over TLS',
				kind: 'tls',
				hostname: 'dns.google',
			},
			// {
			// 	address: 'https://dns.google/dns-query',
			// 	label: 'DNS over HTTPS',
			// },
		],
	},
	{
		name: 'Quad9',
		description: 'Security-focused, threat blocking',
		servers: [
			{ address: '9.9.9.9', label: 'Primary' },
			{ address: '149.112.112.112', label: 'Secondary' },
			{
				address: '9.9.9.9',
				label: 'DNS over TLS',
				kind: 'tls',
				hostname: 'dns.quad9.net',
			},
			// {
			// 	address: 'https://dns.quad9.net/dns-query',
			// 	label: 'DNS over HTTPS',
			// },
		],
	},
	{
		name: 'OpenDNS',
		description: 'Cisco Umbrella network',
		servers: [
			{ address: '208.67.222.222', label: 'Primary' },
			{ address: '208.67.220.220', label: 'Secondary' },
		],
	},
	{
		name: 'AdGuard',
		description: 'Ad-blocking DNS',
		servers: [
			{ address: '94.140.14.14', label: 'Primary' },
			{
				address: '94.140.14.14',
				label: 'DNS over TLS',
				kind: 'tls',
				hostname: 'dns.adguard-dns.com',
			},
			// {
			// 	address: 'https://dns.adguard-dns.com/dns-query',
			// 	label: 'DNS over HTTPS',
			// },
		],
	},
];

export function normalizeEndpoint(input: string, kind: UpstreamKind): string {
	const hp = parseHostPort(input.trim());
	if (!hp) return input.trim();
	const host = hp.host.includes(':') ? `[${hp.host}]` : hp.host;
	const port = hp.port ?? DEFAULT_PORTS[kind];
	return `${host}:${port}`;
}

export function serverUpstream(server: ProviderServer): Upstream {
	const kind = server.kind ?? 'plain';
	const endpoint = normalizeEndpoint(server.address, kind);

	if (kind === 'plain') return { kind, endpoint };

	return server.hostname
		? { kind, endpoint, hostname: server.hostname }
		: { kind, endpoint };
}

export function upstreamKey(upstream: Upstream): string {
	if (upstream.kind === 'plain') return `plain:${upstream.endpoint}`;
	return `tls:${upstream.endpoint}:${upstream.hostname ?? ''}`;
}

export function upstreamsMatch(a: Upstream, b: Upstream): boolean {
	if (a.kind !== b.kind || a.endpoint !== b.endpoint) return false;

	// Same endpoint under a different certificate identity is a different upstream.
	if (a.kind === 'tls' && b.kind === 'tls') {
		return (a.hostname ?? '') === (b.hostname ?? '');
	}

	return true;
}

export function hasUpstream(list: Upstream[], upstream: Upstream): boolean {
	return list.some((u) => upstreamsMatch(u, upstream));
}

export function getProviderGroup(
	upstream: Upstream,
): ProviderGroup | undefined {
	for (const provider of providerGroups) {
		for (const server of provider.servers) {
			if (upstreamsMatch(upstream, serverUpstream(server))) return provider;
		}
	}
}

export type DetectedProtocol = 'UDP/TCP' | 'DoH' | 'DoT';

export function protocolForKind(kind: UpstreamKind): DetectedProtocol {
	if (kind === 'tls') return 'DoT';
	return 'UDP/TCP';
}
