import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { ProtectedRoute } from '../components/ProtectedRoute';
import { DashboardLayout } from '../layouts/DashboardLayout';
import DomainRulesPage from './pages/domain-rules';
import ConfigPage from './pages/config';
import HomePage from './pages/home';
import LocalRecordsPage from './pages/local-records';
import LoginPage from './pages/login';
import LogsPage from './pages/logs';
import NotFoundPage from './pages/not-found';
import SetupPage from './pages/setup';

export function AppRouter() {
	return (
		<BrowserRouter>
			<Routes>
				<Route path='/setup' element={<SetupPage />} />
				<Route element={<ProtectedRoute requiresAuthentication={false} />}>
					<Route path='/' element={<LoginPage />} />
				</Route>
				<Route element={<ProtectedRoute requiresAuthentication />}>
					<Route element={<DashboardLayout />}>
						<Route path='/home' element={<HomePage />} />
						<Route path='/domain-rules' element={<DomainRulesPage />} />
						<Route path='/local-records' element={<LocalRecordsPage />} />
						<Route path='/logs' element={<LogsPage />} />
						<Route path='/config' element={<ConfigPage />} />
					</Route>
				</Route>
				<Route path='*' element={<NotFoundPage />} />
			</Routes>
		</BrowserRouter>
	);
}
