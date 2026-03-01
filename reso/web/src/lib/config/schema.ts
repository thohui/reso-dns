import z from 'zod';

const isValidPort = (p: string) => {
	const n = Number(p);
	return Number.isInteger(n) && n >= 1 && n <= 65535;
};

const isValidIPv4 = (s: string) => {
	const parts = s.split('.');
	if (parts.length !== 4) return false;
	return parts.every((p) => {
		if (!/^\d{1,3}$/.test(p)) return false;
		const n = Number(p);
		return n >= 0 && n <= 255;
	});
};

const isValidHostname = (s: string) => {
	let hostname = s;

	if (hostname.length < 1 || hostname.length > 253) return false;

	// allow trailing dot
	if (s.endsWith('.')) {
		hostname = hostname.slice(0, -1);
	}

	if (s.length < 1) return false;

	const labels = hostname.split('.');

	return labels.every((label) => {
		if (label.length < 1 || label.length > 63) return false;
		if (!/^[a-zA-Z0-9-]+$/.test(label)) return false;
		if (label.startsWith('-') || label.endsWith('-')) return false;
		return true;
	});
};
const isValidIPv6 = (s: string) => {
	try {
		new URL(`http://[${s}]`);
		return true;
	} catch {
		return false;
	}
};

function parseHostPort(input: string): { host: string; port?: string } | null {
	const s = input.trim();
	if (!s) return null;

	// Bracketed IPv6: e.g. [::1]:53 or [::1]
	if (s.startsWith('[')) {
		const end = s.indexOf(']');
		if (end === -1) return null;
		const host = s.slice(1, end);
		const rest = s.slice(end + 1);

		if (!isValidIPv6(host)) return null;

		if (!rest) return { host };
		if (!rest.startsWith(':')) return null;

		const port = rest.slice(1);
		if (!port) return null;
		return { host, port };
	}

	// Non-bracket form: host or host:port

	const colonCount = (s.match(/:/g) || []).length;
	if (colonCount > 1) return null;

	const idx = s.lastIndexOf(':');
	if (idx === -1) return { host: s };

	const host = s.slice(0, idx);
	const port = s.slice(idx + 1);
	if (!host || !port) return null;
	return { host, port };
}

export const UpstreamSpecSchema = z
	.string()
	.trim()
	.min(1, 'Address is empty')
	.superRefine((val, ctx) => {
		// DoH
		// TODO: uncomment when server supports DOH.
		if (val.startsWith('https://') || val.startsWith('http://')) {
			ctx.addIssue({
				code: 'custom',
				message: 'DoH is currently not supported!',
			});
			return;
			// try {
			// 	const u = new URL(val);
			// 	if (u.protocol !== 'https:' && u.protocol !== 'http:') {
			// 		ctx.addIssue({ code: 'custom', message: 'DoH URL must be http(s)' });
			// 	}
			// 	if (!u.hostname) {
			// 		ctx.addIssue({ code: 'custom', message: 'DoH URL missing hostname' });
			// 	}
			// } catch {
			// ctx.addIssue({ code: 'custom', message: 'Invalid DoH URL' });
			// }
			// return;
		}

		// Optional scheme
		let scheme = 'plain';
		let rest = val;

		const split = val.split('://');
		if (split.length === 2) {
			scheme = split[0];
			rest = split[1];
		} else if (split.length > 2) {
			ctx.addIssue({ code: 'custom', message: 'Invalid scheme separator' });
			return;
		}

		const validSchemes = ['plain'];

		if (!validSchemes.includes(scheme)) {
			ctx.addIssue({
				code: 'custom',
				message: `Unsupported scheme: ${scheme}`,
			});
			return;
		}

		const hp = parseHostPort(rest);
		if (!hp) {
			ctx.addIssue({
				code: 'custom',
				message: 'Expected host[:port] (IPv6 must be in brackets)',
			});
			return;
		}

		const hostOk =
			isValidIPv4(hp.host) || isValidHostname(hp.host) || isValidIPv6(hp.host);
		if (!hostOk) {
			ctx.addIssue({ code: 'custom', message: 'Invalid host' });
			return;
		}

		if (hp.port && !isValidPort(hp.port)) {
			ctx.addIssue({ code: 'custom', message: 'Invalid port (1-65535)' });
			return;
		}
	});
