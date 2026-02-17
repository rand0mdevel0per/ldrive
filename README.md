# LDrive - Linux.DO 社区分布式文件存储系统

<div align="center">

![LDrive Logo](docs/public/logo.svg)

**安全、去中心化的文件存储解决方案**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/typescript-5.0+-blue.svg)](https://www.typescriptlang.org)

[快速开始](#快速开始) • [文档](docs/) • [演示](https://873389f6.ldrive-web.pages.dev)

</div>

## 📖 项目简介

LDrive 是为 Linux.DO 社区打造的分布式文件存储系统，通过社区成员贡献存储和带宽节点，使用 LDC 积分激励参与者。

### 核心特性

- 🔐 **LinuxDO OAuth 认证** - 使用社区账号一键登录
- 💰 **LDC 积分系统** - 1 LDC = 1 Credit，透明计费
- 🔑 **设备 Token 管理** - 安全的节点认证机制
- 🌐 **P2P 网络** - QUIC + WSS 双协议，自动 NAT 穿透
- 📦 **智能分块** - FastCDC + BLAKE3，高效去重
- 🛡️ **纠删码** - Reed-Solomon 4+2，容忍 2 节点故障
- ⚡ **边缘加速** - Cloudflare Workers 全球 CDN

## 🚀 快速开始

### 用户端

1. 访问 [LDrive Web](https://873389f6.ldrive-web.pages.dev)
2. 使用 Linux.DO 账号登录
3. 充值积分（1 LDC = 1 Credit）
4. 生成设备 Token
5. 上传/下载文件

### 节点部署

#### Linux/macOS
```bash
curl -fsSL https://raw.githubusercontent.com/rand0mdevel0per/ldrive/main/scripts/install.sh | bash
```

#### Windows
```powershell
curl -o install.bat https://raw.githubusercontent.com/rand0mdevel0per/ldrive/main/scripts/install.bat && install.bat
```

#### 启动存储节点
```bash
ldrive-node storage \
  --token YOUR_DEVICE_TOKEN \
  --storage-path ~/.ldrive/data \
  --quota 50GB
```

## 💰 计费标准

| 项目 | 价格 |
|------|------|
| 存储 | 0.5 Credits/GB |
| 流量 | 0.001 Credits/MB |
| 充值 | 1 LDC = 1 Credit |
| 平台抽成 | 5% |

## 🏗️ 架构设计

```
┌─────────────────────────────────────────┐
│  Web UI (React + Vite)                  │
│  ↓                                      │
│  CF Workers Gateway                     │
│  ↓                                      │
│  P2P Network (QUIC/WSS)                 │
│  ↓                                      │
│  Storage Nodes (Rust)                   │
│  - FastCDC 分块                         │
│  - BLAKE3 哈希                          │
│  - Reed-Solomon 纠删码                  │
│  - redb 本地存储                        │
└─────────────────────────────────────────┘
```

详细架构请查看 [计划文档](~/.claude/plans/playful-waddling-muffin.md)


## 🛠️ 开发指南

### 前置要求

- Rust 1.70+
- Node.js 18+
- Cloudflare 账号

### 本地开发

```bash
# 克隆仓库
git clone https://github.com/rand0mdevel0per/ldrive.git
cd ldrive

# 前端开发
cd app/web
npm install
npm run dev

# Worker 开发
cd worker
npm install
npx wrangler dev

# Rust 节点开发
cargo build --release
```

### 项目结构

```
ldrive/
├── crates/          # Rust 核心库
│   ├── ldrive-node/ # 节点二进制
│   ├── ldrive-net/  # 网络层
│   ├── ldrive-dht/  # DHT 实现
│   └── ...
├── app/
│   ├── web/         # React 前端
│   └── server/      # 应用服务器
├── worker/          # CF Worker 网关
├── scripts/         # 安装脚本
└── docs/            # VitePress 文档
```


## 📡 API 文档

### REST API (Worker)

```bash
# 充值积分
POST /recharge
Body: { "amount": 10 }
Response: { "payUrl": "...", "outTradeNo": "...", "credits": 10 }

# 下载文件
GET /file/:hash
Headers: X-User-ID: <user_id>
Response: File stream

# 查看指标
GET /admin/metrics?date=2026-02-17
Response: { "uploads": 100, "downloads": 200, "bandwidth": 1024000, "errors": 5 }
```

详细 API 文档请查看 [docs/api/rest.md](docs/api/rest.md)

## 🔗 在线服务

- **前端**: https://873389f6.ldrive-web.pages.dev
- **Worker**: https://ldrive-worker.rand0mk4cas.workers.dev
- **文档**: [docs/](docs/)
- **GitHub**: https://github.com/rand0mdevel0per/ldrive

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing`)
5. 提交 Pull Request

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE)

## 🙏 致谢

- [Linux.DO](https://linux.do) 社区
- [Cloudflare Workers](https://workers.cloudflare.com)
- [IPFS](https://ipfs.io) 和 [iroh](https://github.com/n0-computer/iroh) 项目启发

---

Made with ❤️ for Linux.DO Community
