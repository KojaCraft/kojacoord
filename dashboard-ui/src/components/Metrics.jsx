import { useEffect, useState } from 'react'
import axios from 'axios'
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts'

const Metrics = () => {
  const [metrics, setMetrics] = useState([])

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const response = await axios.get('/api/metrics')
        const data = {
          timestamp: new Date().toLocaleTimeString(),
          connections: response.data.active_connections || 0,
          packets: response.data.packets_relayed || 0,
        }
        setMetrics(prev => [...prev.slice(-19), data])
      } catch (error) {
        console.error('Failed to fetch metrics:', error)
      }
    }

    fetchMetrics()
    const interval = setInterval(fetchMetrics, 2000)
    return () => clearInterval(interval)
  }, [])

  return (
    <div>
      <h2 className="text-3xl font-bold text-white mb-6">Metrics</h2>
      
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h3 className="text-xl font-semibold text-white mb-4">Active Connections</h3>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" />
                <YAxis stroke="#9ca3af" />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151' }}
                  itemStyle={{ color: '#fff' }}
                />
                <Area type="monotone" dataKey="connections" stroke="#3b82f6" fill="#3b82f6" fillOpacity={0.3} />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h3 className="text-xl font-semibold text-white mb-4">Packets Relayed</h3>
          <div className="h-64">
            <ResponsiveContainer width="100%" height="100%">
              <LineChart data={metrics}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" />
                <YAxis stroke="#9ca3af" />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151' }}
                  itemStyle={{ color: '#fff' }}
                />
                <Line type="monotone" dataKey="packets" stroke="#10b981" strokeWidth={2} />
              </LineChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      <div className="mt-6 grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h4 className="text-gray-400 text-sm mb-2">Total Connections</h4>
          <p className="text-3xl font-bold text-white">
            {metrics.length > 0 ? metrics[metrics.length - 1].connections : 0}
          </p>
        </div>
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h4 className="text-gray-400 text-sm mb-2">Packets/Second</h4>
          <p className="text-3xl font-bold text-white">
            {metrics.length > 0 ? metrics[metrics.length - 1].packets : 0}
          </p>
        </div>
        <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
          <h4 className="text-gray-400 text-sm mb-2">Uptime</h4>
          <p className="text-3xl font-bold text-white">99.9%</p>
        </div>
      </div>
    </div>
  )
}

export default Metrics
