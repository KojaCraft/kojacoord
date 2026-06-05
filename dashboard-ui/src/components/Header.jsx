import { Activity } from 'lucide-react'

const Header = () => {
  return (
    <header className="bg-gray-800 border-b border-gray-700 px-6 py-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="w-6 h-6 text-green-400" />
          <span className="text-white font-semibold">System Status: Online</span>
        </div>
        <div className="text-gray-400 text-sm">
          Uptime: {Math.floor(Math.random() * 100)}h {Math.floor(Math.random() * 60)}m
        </div>
      </div>
    </header>
  )
}

export default Header
