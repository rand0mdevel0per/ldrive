import { useState, useEffect } from 'react'
import { Wallet, Key, LogIn, LogOut, Upload, Download } from 'lucide-react'

const WORKER_URL = 'https://ldrive-worker.rand0mk4cas.workers.dev'
const CLIENT_ID = 'kXMWzKAsXA0RvveyblrjNE6cnCn6hcfn'

export default function App() {
  const [user, setUser] = useState(null)
  const [balance, setBalance] = useState(0)
  const [deviceToken, setDeviceToken] = useState('')
  const [webdavCreds, setWebdavCreds] = useState(null)

  useEffect(() => {
    const token = localStorage.getItem('ldrive_token')
    if (token) {
      setUser({ token })
      fetchBalance(token)
    }

    const savedDeviceToken = localStorage.getItem('device_token')
    if (savedDeviceToken) setDeviceToken(savedDeviceToken)

    const savedWebdav = localStorage.getItem('webdav_creds')
    if (savedWebdav) setWebdavCreds(JSON.parse(savedWebdav))

    const code = new URLSearchParams(window.location.search).get('code')
    if (code) handleOAuthCallback(code)
  }, [])

  const handleOAuthCallback = async (code) => {
    try {
      const res = await fetch(`${WORKER_URL}/oauth/token`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code, redirect_uri: window.location.origin })
      })
      const data = await res.json()
      if (data.access_token) {
        localStorage.setItem('ldrive_token', data.access_token)
        const userRes = await fetch(`${WORKER_URL}/oauth/user`, {
          headers: { Authorization: `Bearer ${data.access_token}` }
        })
        const userData = await userRes.json()
        setUser(userData)
        fetchBalance(userData.id)
      }
    } catch (e) {
      console.error('OAuth failed:', e)
    }
    window.history.replaceState({}, '', '/')
  }

  const fetchBalance = async (userId) => {
    try {
      const token = localStorage.getItem('ldrive_token')
      const res = await fetch(`${WORKER_URL}/balance/${userId}`, {
        headers: { Authorization: `Bearer ${token}` }
      })
      const data = await res.json()
      setBalance(data.balance || 0)
    } catch (e) {
      setBalance(0)
    }
  }

  const login = () => {
    const redirectUri = window.location.origin
    window.location.href = `https://connect.linux.do/oauth2/authorize?client_id=${CLIENT_ID}&redirect_uri=${redirectUri}&response_type=code&scope=openid profile`
  }

  const logout = () => {
    localStorage.removeItem('ldrive_token')
    setUser(null)
    setBalance(0)
  }

  const recharge = async () => {
    const amount = prompt('充值金额 (LDC):', '10')
    if (!amount) return

    try {
      const res = await fetch(`${WORKER_URL}/recharge`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ amount: parseFloat(amount) })
      })
      const data = await res.json()
      if (data.payUrl) window.open(data.payUrl, '_blank')
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
            <div className="card">
              <h3><Wallet size={20} /> 积分管理</h3>
              <p>当前余额: <strong>{balance.toFixed(2)} Credits</strong></p>
              <p className="pricing">存储: 0.5 Credits/GB · 流量: 0.001 Credits/MB</p>
              <button onClick={recharge} className="btn-primary">充值</button>
            </div>

            <div className="card">
              <h3><Key size={20} /> 设备 Token</h3>
              <p>用于节点认证的设备令牌</p>
              {deviceToken ? (
                <div className="token-display">
                  <code>{deviceToken.slice(0, 32)}...</code>
                  <button onClick={() => navigator.clipboard.writeText(deviceToken)} className="btn-secondary">复制</button>
                </div>
              ) : (
                <button onClick={generateToken} className="btn-primary">生成 Token</button>
              )}
            </div>

            <div className="card">
              <h3><Upload size={20} /> WebDAV 访问</h3>
              <p>用于文件上传和管理</p>
              {webdavCreds ? (
                <div>
                  <p><strong>用户名:</strong> {webdavCreds.username}</p>
                  <div className="token-display">
                    <code>{webdavCreds.password}</code>
                    <button onClick={() => navigator.clipboard.writeText(webdavCreds.password)} className="btn-secondary">复制</button>
                  </div>
                  <p style={{marginTop: '1rem', fontSize: '0.9rem'}}>地址: {WORKER_URL}/dav/</p>
                </div>
              ) : (
                <button onClick={setupWebDAV} className="btn-primary">设置 WebDAV</button>
              )}
            </div>

            <div className="card">
              <h3><Upload size={20} /> 快速开始</h3>
              <p>安装存储节点:</p>
              <code className="install-cmd">curl -fsSL https://raw.githubusercontent.com/rand0mdevel0per/ldrive/main/scripts/install.sh | bash</code>
              <p style={{marginTop: '1rem'}}>启动节点:</p>
              <code className="install-cmd">ldrive-node storage --token {deviceToken || 'YOUR_TOKEN'} --quota 50GB</code>
            </div>
          </div>
        )}
      </main>
    </div>
  )
}
