import { Navigate, Outlet } from 'react-router-dom';
import { useIsAuthenticated } from '../hooks/useIsAuthenticated';
import { useIsSetupRequired } from '../hooks/useIsSetupRequired';

interface Props {
	requiresAuthentication: boolean;
}

export function ProtectedRoute({ requiresAuthentication }: Props) {
	const authenticated = useIsAuthenticated();
	const setupRequired = useIsSetupRequired();

	if (setupRequired) {
		return <Navigate to='/setup' replace />;
	}

	if (requiresAuthentication && !authenticated) {
		return <Navigate to='/' replace />;
	}

	if (!requiresAuthentication && authenticated) {
		return <Navigate to='/home' replace />;
	}

	return <Outlet />;
}
