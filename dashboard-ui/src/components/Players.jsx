import { useEffect, useState } from 'react'
import axios from 'axios'
import { Search, Shield, AlertCircle } from 'lucide-react'

const Players = () => {
  const [players, setPlayers] = useState([])
  const [searchTerm, setSearchTerm] = useState('')

  useEffect(() => {
    const fetchPlayers = async () => {
      try {
        const response = await axios.get('/api/players')
        setPlayers(response.data.players || [])
      } catch (error) {
        console.error('Failed to fetch players:', error)
      }
    }

    fetchPlayers()
    const interval = setInterval(fetchPlayers, 5000)
    return () => clearInterval(interval)
  }, [])

  const filteredPlayers = players.filter(player =>
    player.username?.toLowerCase().includes(searchTerm.toLowerCase()) ||
    player.server?.toLowerCase().includes(searchTerm.toLowerCase())
  )

  return (
    <div>
      <h2 className="text-3xl font-bold text-white mb-6">Players</h2>
      
      <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 mb-6">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-5 h-5" />
          <input
            type="text"
            placeholder="Search players..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full bg-gray-700 text-white pl-10 pr-4 py-2 rounded-lg border border-gray-600 focus:outline-none focus:border-blue-500"
          />
        </div>
      </div>

      <div className="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden">
        <table className="w-full">
          <thead className="bg-gray-700">
            <tr>
              <th className="text-left text-gray-300 font-semibold px-6 py-3">Username</th>
              <th className="text-left text-gray-300 font-semibold px-6 py-3">UUID</th>
              <th className="text-left text-gray-300 font-semibold px-6 py-3">Server</th>
              <th className="text-left text-gray-300 font-semibold px-6 py-3">Protocol</th>
              <th className="text-left text-gray-300 font-semibold px-6 py-3">Status</th>
            </tr>
          </thead>
          <tbody>
            {filteredPlayers.map((player) => (
              <tr key={player.uuid} className="border-t border-gray-700">
                <td className="px-6 py-4 text-white">{player.username}</td>
                <td className="px-6 py-4 text-gray-400 font-mono text-sm">
                  {player.uuid?.substring(0, 8)}...
                </td>
                <td className="px-6 py-4 text-gray-300">{player.server}</td>
                <td className="px-6 py-4 text-gray-400">{player.protocol_version}</td>
                <td className="px-6 py-4">
                  <span className="flex items-center gap-2 text-green-400">
                    <Shield className="w-4 h-4" />
                    Online
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

export default Players
