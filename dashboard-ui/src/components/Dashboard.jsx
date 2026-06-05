import { useEffect, useState } from 'react'
import axios from 'axios'
import { Users, Server, Activity, AlertTriangle } from 'lucide-react'

const Dashboard = () => {
  const [stats, setStats] = useState({
    totalConnections: 0,
    activeConnections: 0,
    serversOnline: 0,
    violations: 0,
  })

  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await axios.get('/api/metrics')
        setStats({
          totalConnections: response.data.total_connections || 0,
          activeConnections: response.data.active_connections || 0,
          serversOnline: 3,
          violations: Math.floor(Math.random() * 10),
        })
      } catch (error) {
        console.error('Failed to fetch stats:', error)
      }
    }

    fetchStats()
    const interval = setInterval(fetchStats, 5000)
    return () => clearInterval(interval)
  }, [])

  const statCards = [
    {
      title: 'Active Players',
      value: stats.activeConnections,
      icon: Users,
      color: 'text-blue-400',
      bgColor: 'bg-blue-400/10',
    },
    {
      title: 'Total Connections',
      value: stats.totalConnections,
      icon: Activity,
      color: 'text-green-400',
      bgColor: 'bg-green-400/10',
    },
    {
      title: 'Servers Online',
      value: stats.serversOnline,
      icon: Server,
      color: 'text-purple-400',
      bgColor: 'bg-purple-400/10',
    },
    {
      title: 'Violations',
      value: stats.violations,
      icon: AlertTriangle,
      color: 'text-red-400',
      bgColor: 'bg-red-400/10',
    },
  ]

  return (
    <div>
      <h2 className="text-3xl font-bold text-white mb-6">Dashboard</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
        {statCards.map((stat) => (
          <div key={stat.title} className="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-gray-400 text-sm">{stat.title}</p>
                <p className="text-3xl font-bold text-white mt-2">{stat.value}</p>
              </div>
              <div className={`p-3 rounded-lg ${stat.bgColor}`}>
                <stat.icon className={`w-8 h-8 ${stat.color}`} />
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h3 className="text-xl font-semibold text-white mb-4">Recent Activity</h3>
          <div className="space-y-3">
            {[1, 2, 3, 4, 5].map((i) => (
              <div key={i} className="flex items-center gap-3 text-gray-300">
                <div className="w-2 h-2 rounded-full bg-green-400" />
                <span>Player {i} joined server lobby</span>
                <span className="text-gray-500 text-sm ml-auto">{i}m ago</span>
              </div>
            ))}
          </div>
        </div>

        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h3 className="text-xl font-semibold text-white mb-4">Server Status</h3>
          <div className="space-y-3">
            {['Lobby', 'Survival', 'Creative'].map((server) => (
              <div key={server} className="flex items-center justify-between text-gray-300">
                <span>{server}</span>
                <div className="flex items-center gap-2">
                  <div className="w-2 h-2 rounded-full bg-green-400" />
                  <span className="text-sm">{Math.floor(Math.random() * 20)}/100</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

export default Dashboard
