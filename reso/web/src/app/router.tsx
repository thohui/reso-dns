import { BrowserRouter, Route, Routes } from 'react-router-dom';
import { ProtectedRoute } from '../components/ProtectedRoute';
import { DashboardLayout } from '../layouts/DashboardLayout';
import BlocklistPage from './pages/blocklist';
import HomePage from './pages/home';
import LoginPage from './pages/login';
import LogsPage from './pages/logs';

export function AppRouter() {
	return (
		<BrowserRouter>
			<Routes>
				<Route element={<ProtectedRoute requiresAuthentication={false} />}>
					<Route path='/' element={<LoginPage />} />
				</Route>
				<Route element={<ProtectedRoute requiresAuthentication />}>
					<Route element={<DashboardLayout />}>
						<Route path='/home' element={<HomePage />} />
						<Route path='/blocklist' element={<BlocklistPage />} />
						<Route path='/logs' element={<LogsPage />} />
					</Route>
				</Route>
			</Routes>
		</BrowserRouter>
	);
}
