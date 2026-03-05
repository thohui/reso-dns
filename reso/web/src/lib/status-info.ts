import {
	AlertTriangle,
	CheckCircle,
	type LucideIcon,
	ShieldOff,
	XCircle,
	Zap,
} from 'lucide-react';
import {
	type Activity,
	getErrorTypeLabel,
	type QueryActivity,
} from './api/activity';

export interface StatusInfo {
	label: string;
	color: string;
	bg: string;
	icon: LucideIcon;
	text?: string;
}

export function getStatusInfo(activity: Activity): StatusInfo {
	if (activity.kind === 'error') {
		return {
			label: 'error',
			color: 'status.error',
			bg: 'status.errorMuted',
			icon: XCircle,
			text: getErrorTypeLabel(activity.d.error_type),
		};
	}

	const q = activity as QueryActivity;

	if (q.d.blocked) {
		return {
			label: 'blocked',
			color: 'status.blocked',
			bg: 'status.blockedMuted',
			icon: ShieldOff,
			text: 'Blocked by filter',
		};
	}



	if (q.d.cache_hit) {
		return {
			label: 'cached',
			color: 'status.cached',
			bg: 'status.cachedMuted',
			icon: Zap,
			text: 'Served from cache',
		};
	}

	if (q.d.rate_limited) {
		return {
			label: 'rate limited',
			color: 'status.rate_limited',
			bg: 'status.rate_limitedMuted',
			icon: AlertTriangle,
			text: 'Rate limited',
		};
	}

	if (q.d.rcode !== 0) {
		return {
			label: 'warning',
			color: 'status.warn',
			bg: 'status.warnMuted',
			icon: AlertTriangle,
		};
	}

	return {
		label: 'ok',
		color: 'status.success',
		bg: 'status.successMuted',
		icon: CheckCircle,
	};
}
