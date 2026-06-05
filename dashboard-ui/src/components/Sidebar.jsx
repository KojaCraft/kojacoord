import { NavLink } from 'react-router-dom'
import { 
  LayoutDashboard, 
  Users, 
  Server, 
  BarChart3 
} from 'lucide-react'

const Sidebar = () => {
  const navItems = [
    { path: '/dashboard', icon: LayoutDashboard, label: 'Dashboard' },
    { path: '/players', icon: Users, label: 'Players' },
    { path: '/servers', icon: Server, label: 'Servers' },
    { path: '/metrics', icon: BarChart3, label: 'Metrics' },
  ]

  return (
    <aside className="w-64 bg-gray-800 border-r border-gray-700">
      <div className="p-6">
        <h1 className="text-2xl font-bold text-white">Kojacoord</h1>
        <p className="text-sm text-gray-400 mt-1">Proxy Dashboard</p>
      </div>
      <nav className="mt-6">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `flex items-center gap-3 px-6 py-3 text-gray-300 hover:bg-gray-700 hover:text-white transition-colors ${
                isActive ? 'bg-gray-700 text-white' : ''
              }`
            }
          >
            <item.icon className="w-5 h-5" />
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  )
}

export default Sidebar
