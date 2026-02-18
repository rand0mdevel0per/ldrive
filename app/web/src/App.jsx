import { useState, useEffect } from 'react'
import { Wallet, Key, LogIn, LogOut, Upload, Download, TrendingUp, HardDrive, Activity } from 'lucide-react'
import { LineChart, Line, PieChart, Pie, Cell, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts'

const WORKER_URL = 'https://ldrive-worker.rand0mk4cas.workers.dev'
const CLIENT_ID = 'kXMWzKAsXA0RvveyblrjNE6cnCn6hcfn'

export default function App() {
  const [user, setUser] = useState(null)
  const [balance, setBalance] = useState(0)
  const [history, setHistory] = useState([])
  const [deviceToken, setDeviceToken] = useState('')
  const [webdavCreds, setWebdavCreds] = useState(null)
  const [os, setOs] = useState('linux')

  useEffect(() => {
    const savedUser = localStorage.getItem('ldrive_user')
    if (savedUser) {
      const userData = JSON.parse(savedUser)
      setUser(userData)
      fetchBalance(userData.id)
    }

    const savedDeviceToken = localStorage.getItem('device_token')
    if (savedDeviceToken) setDeviceToken(savedDeviceToken)

    const savedWebdav = localStorage.getItem('webdav_creds')
    if (savedWebdav) setWebdavCreds(JSON.parse(savedWebdav))

    const ua = navigator.userAgent.toLowerCase()
    if (ua.includes('win')) setOs('windows')
    else if (ua.includes('mac')) setOs('macos')
    else setOs('linux')

    const params = new URLSearchParams(window.location.search)
    const code = params.get('code')
    const order = params.get('order')

    if (code) handleOAuthCallback(code)

    const pendingOrder = localStorage.getItem('pending_order')
    if (pendingOrder && !code) {
      handlePaymentCallback(pendingOrder)
    }
  }, [])

  const handleOAuthCallback = async (code) => {
    try {
      const res = await fetch(`${WORKER_URL}/oauth/token`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code, redirect_uri: window.location.origin })
      })
      const data = await res.json()

      if (data.error) {
        alert('登录失败: ' + (data.error_description || data.error))
        return
      }

      if (data.access_token) {
        localStorage.setItem('ldrive_token', data.access_token)
        const userRes = await fetch(`${WORKER_URL}/oauth/user`, {
          headers: { Authorization: `Bearer ${data.access_token}` }
        })
        const userData = await userRes.json()
        localStorage.setItem('ldrive_user', JSON.stringify(userData))
        setUser(userData)
        fetchBalance(userData.id)
      } else {
        alert('登录失败: 未获取到 access_token')
      }
    } catch (e) {
      alert('登录失败: ' + e.message)
    } finally {
      window.history.replaceState({}, '', '/')
    }
  }

  const handlePaymentCallback = async (outTradeNo) => {
    try {
      const token = localStorage.getItem('ldrive_token')
      const res = await fetch(`${WORKER_URL}/ldc/confirm`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify({ outTradeNo })
      })
      const data = await res.json()
      if (data.success) {
        setBalance(data.balance)
        localStorage.removeItem('pending_order')
        alert('充值成功！')
      } else {
        alert('充值确认失败')
      }
    } catch (e) {
      alert('确认支付失败: ' + e.message)
    } finally {
      window.history.replaceState({}, '', '/')
    }
  }

  const fetchBalance = async (userId) => {
    try {
      const token = localStorage.getItem('ldrive_token')
      const res = await fetch(`${WORKER_URL}/balance/${userId}`, {
        headers: { Authorization: `Bearer ${token}` }
      })
      const data = await res.json()
      setBalance(data.balance || 0)

      const histRes = await fetch(`${WORKER_URL}/history/${userId}`)
      const histData = await histRes.json()
      setHistory(histData.history || [])
    } catch (e) {
      setBalance(0)
      setHistory([])
    }
  }

  const login = () => {
    const redirectUri = window.location.origin
    window.location.href = `https://connect.linux.do/oauth2/authorize?client_id=${CLIENT_ID}&redirect_uri=${redirectUri}&response_type=code&scope=openid profile`
  }

  const logout = () => {
    localStorage.removeItem('ldrive_token')
    localStorage.removeItem('ldrive_user')
    setUser(null)
    setBalance(0)
  }

  const recharge = async () => {
    const amount = prompt('充值金额 (LDC):', '10')
    if (!amount) return

    try {
      const token = localStorage.getItem('ldrive_token')
      const res = await fetch(`${WORKER_URL}/recharge`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`
        },
        body: JSON.stringify({ amount: parseFloat(amount) })
      })
      const data = await res.json()
      if (res.status === 401 && data.needsLogin) {
        alert('登录已过期，请重新登录')
        logout()
        login()
        return
      }
      if (data.payUrl) {
        localStorage.setItem('pending_order', data.outTradeNo)
        window.location.href = data.payUrl
      }
    } catch (e) {
      alert('充值失败: ' + e.message)
    }
  }

  const generateToken = () => {
    const token = Array.from(crypto.getRandomValues(new Uint8Array(32)))
      .map(b => b.toString(16).padStart(2, '0')).join('')
    setDeviceToken(token)
    localStorage.setItem('device_token', token)
  }

  const setupWebDAV = async () => {
    const token = localStorage.getItem('ldrive_token')
    const res = await fetch(`${WORKER_URL}/webdav/setup`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${token}` }
    })
    const data = await res.json()
    setWebdavCreds(data)
    localStorage.setItem('webdav_creds', JSON.stringify(data))
  }

  return (
    <div className="app">
      <header>
        <div className="logo">
          <svg width="40" height="40" viewBox="0 0 100 100">
            <polygon points="50,10 90,30 90,70 50,90 10,70 10,30" fill="url(#grad)" />
            <defs>
              <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
                <stop offset="0%" stopColor="#8b5cf6" />
                <stop offset="100%" stopColor="#6366f1" />
              </linearGradient>
            </defs>
          </svg>
          <h1>LDrive</h1>
        </div>
        {user ? (
          <div className="user-info">
            <span className="balance"><Wallet size={16} /> {balance.toFixed(2)} Credits</span>
            <button onClick={logout} className="btn-secondary"><LogOut size={16} /> 退出</button>
          </div>
        ) : (
          <button onClick={login} className="btn-primary"><LogIn size={16} /> 登录</button>
        )}
      </header>

      <main>
        {!user ? (
          <div className="hero">
            <h2>Linux.DO 社区分布式存储</h2>
            <p>安全、去中心化的文件存储解决方案</p>
            <button onClick={login} className="btn-large">开始使用</button>
          </div>
        ) : (
          <div className="dashboard">
            <div className="stats-grid">
              <div className="stat-card">
                <Wallet size={24} />
                <div>
                  <p className="stat-label">余额</p>
                  <p className="stat-value">{balance.toFixed(2)} LDC</p>
                </div>
              </div>
              <div className="stat-card">
                <Activity size={24} />
                <div>
                  <p className="stat-label">节点数</p>
                  <p className="stat-value">0</p>
                </div>
              </div>
              <div className="stat-card">
                <HardDrive size={24} />
                <div>
                  <p className="stat-label">存储</p>
                  <p className="stat-value">0 GB</p>
                </div>
              </div>
              <div className="stat-card">
                <TrendingUp size={24} />
                <div>
                  <p className="stat-label">流量</p>
                  <p className="stat-value">0 MB</p>
                </div>
              </div>
            </div>

            <div className="charts-grid">
              <div className="chart-card">
                <h3>余额趋势</h3>
                {history.length > 0 ? (
                  <ResponsiveContainer width="100%" height={200}>
                    <LineChart data={history.map(h => ({ time: new Date(h.time).toLocaleDateString(), balance: h.balance }))}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="time" />
                      <YAxis />
                      <Tooltip />
                      <Line type="monotone" dataKey="balance" stroke="#8b5cf6" />
                    </LineChart>
                  </ResponsiveContainer>
                ) : (
                  <div style={{height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#9ca3af'}}>
                    暂无数据
                  </div>
                )}
              </div>
              <div className="chart-card">
                <h3>存储容量</h3>
                <div style={{height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#9ca3af'}}>
                  暂无数据
                </div>
              </div>
            </div>

            <div className="actions-grid">
              <div className="card">
                <h3><Wallet size={20} /> 充值</h3>
                <button onClick={recharge} className="btn-primary">充值 LDC</button>
              </div>
              <div className="card">
                <h3><Key size={20} /> 设备 Token</h3>
                {deviceToken ? (
                  <div className="token-display">
                    <code>{deviceToken.slice(0, 32)}...</code>
                    <button onClick={() => navigator.clipboard.writeText(deviceToken)} className="btn-secondary">复制</button>
                  </div>
                ) : (
                  <button onClick={generateToken} className="btn-primary">生成</button>
                )}
              </div>
              <div className="card">
                <h3><Upload size={20} /> WebDAV</h3>
                <p style={{fontSize: '0.9rem', marginBottom: '1rem'}}>
                  <a href="https://master.ldrive-docs.pages.dev/webdav" target="_blank" rel="noopener" style={{color: '#8b5cf6'}}>使用教程</a>
                </p>
                {webdavCreds ? (
                  <div className="token-display">
                    <code>{webdavCreds.password.slice(0, 16)}...</code>
                    <button onClick={() => navigator.clipboard.writeText(webdavCreds.password)} className="btn-secondary">复制</button>
                  </div>
                ) : (
                  <button onClick={setupWebDAV} className="btn-primary">设置</button>
                )}
              </div>
            </div>

            <div className="card" style={{marginTop: '1.5rem'}}>
              <h3><Download size={20} /> 快速开始</h3>
              <p>安装存储节点:</p>
              {os === 'windows' ? (
                <code className="install-cmd">irm https://github.com/rand0mdevel0per/ldrive/raw/refs/heads/master/scripts/install.ps1 | iex</code>
              ) : (
                <code className="install-cmd">curl -fsSL https://github.com/rand0mdevel0per/ldrive/raw/refs/heads/master/scripts/install.sh | bash</code>
              )}
              <p style={{marginTop: '1rem'}}>启动节点:</p>
              <code className="install-cmd">ldrive-node serve --storage-path ~/.ldrive/data --quota 50GB --listen 0.0.0.0:4433</code>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
