import { useEffect, useState } from 'react'
import axios from 'axios'
import { Server, Activity, Power } from 'lucide-react'

const Servers = () => {
  const [servers, setServers] = useState([])

  useEffect(() => {
    const fetchServers = async () => {
      try {
        const response = await axios.get('/api/servers')
        setServers(response.data.servers || [])
      } catch (error) {
        console.error('Failed to fetch servers:', error)
      }
    }

    fetchServers()
    const interval = setInterval(fetchServers, 5000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div>
      <h2 className="text-3xl font-bold text-white mb-6">Servers</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {servers.map((server) => (
          <div key={server.name} className="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div className="flex items-start justify-between mb-4">
              <div className="flex items-center gap-3">
                <div className="p-3 rounded-lg bg-purple-400/10">
                  <Server className="w-6 h-6 text-purple-400" />
                </div>
                <div>
                  <h3 className="text-white font-semibold">{server.name}</h3>
                  <p className="text-gray-400 text-sm">{server.address}</p>
                </div>
              </div>
              <div className={`p-2 rounded-lg ${server.online ? 'bg-green-400/10' : 'bg-red-400/10'}`}>
                <Activity className={`w-5 h-5 ${server.online ? 'text-green-400' : 'text-red-400'}`} />
              </div>
            </div>
            
            <div className="space-y-3">
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Players</span>
                <span className="text-white">{server.player_count || 0}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-400">Status</span>
                <span className={server.online ? 'text-green-400' : 'text-red-400'}>
                  {server.online ? 'Online' : 'Offline'}
                </span>
              </div>
              <div className="flex gap-2 mt-4">
                <button className="flex-1 bg-blue-600 hover:bg-blue-700 text-white py-2 px-4 rounded-lg transition-colors flex items-center justify-center gap-2">
                  <Power className="w-4 h-4" />
                  Restart
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

export default Servers
