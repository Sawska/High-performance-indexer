import { BrowserRouter, Route, Routes } from 'react-router'
import { AuthProvider } from './auth'
import { Activity } from './pages/Activity'
import { Layout } from './pages/Layout'
import { Login } from './pages/Login'
import { Market } from './pages/Market'
import { Markets } from './pages/Markets'
import { Overview } from './pages/Overview'
import { Status } from './pages/Status'
import { Trader } from './pages/Trader'
import { Traders } from './pages/Traders'
import { Trades } from './pages/Trades'

export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/login" element={<Login mode="login" />} />
          <Route path="/register" element={<Login mode="register" />} />
          <Route element={<Layout />}>
            <Route index element={<Overview />} />
            <Route path="markets" element={<Markets />} />
            <Route path="markets/:id" element={<Market />} />
            <Route path="traders" element={<Traders />} />
            <Route path="traders/:wallet" element={<Trader />} />
            <Route path="trades" element={<Trades />} />
            <Route path="activity" element={<Activity />} />
            <Route path="indexer" element={<Status />} />
            <Route path="*" element={<div className="empty">Page not found.</div>} />
          </Route>
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  )
}
