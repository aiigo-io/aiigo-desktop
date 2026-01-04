import { createBrowserRouter, RouterProvider } from 'react-router-dom';

import { AppLayout } from './components/layout/AppLayout';
import Dashboard from './pages/Dashboard';
import Portfolio from './pages/Portfolio';
import Transactions from './pages/Transactions';
import Swap from './pages/Swap';
import NotFound from './pages/NotFound';

const router = createBrowserRouter([
  {
    path: '/',
    element: <AppLayout />,
    children: [
      { path: '/', element: <Dashboard /> },
      { path: '/portfolio', element: <Portfolio /> },
      { path: '/transactions', element: <Transactions /> },
      { path: '/swap', element: <Swap /> },
      { path: '/*', element: <NotFound /> },
    ],
  },
]);

function App() {
  return <RouterProvider router={router} />;
}

export default App;