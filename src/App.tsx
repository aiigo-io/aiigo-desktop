import { createBrowserRouter, RouterProvider } from 'react-router-dom';

import { AppLayout } from './components/layout/AppLayout';
import Dashboard from './pages/Dashboard';
import NotFound from './pages/NotFound';

const router = createBrowserRouter([
  {
    path: '/',
    element: <AppLayout />,
    children: [
      { path: '/', element: <Dashboard /> },
      { path: '/*', element: <NotFound /> },
    ],
  },
]);

function App() {
  return <RouterProvider router={router} />;
}

export default App;