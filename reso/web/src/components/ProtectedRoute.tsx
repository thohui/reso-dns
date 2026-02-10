import { Navigate, Outlet } from 'react-router-dom';
import { useIsAuthenticated } from '../hooks/useIsAuthenticated';

interface Props {
	requiresAuthentication: boolean;
}

export function ProtectedRoute({ requiresAuthentication }: Props) {
	const authenticated = useIsAuthenticated();

	if (requiresAuthentication && !authenticated) {
		return <Navigate to="/" replace />;
	}

	if (!requiresAuthentication && authenticated) {
		return <Navigate to="/home" replace />;
	}

	return <Outlet />;
}
