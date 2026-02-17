---
layout: home

hero:
  name: LDrive
  text: 分布式文件存储系统
  tagline: Linux.DO 社区驱动的去中心化存储方案
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/rand0mdevel0per/ldrive

features:
  - icon: 🔒
    title: 后量子加密
    details: 使用 ML-KEM-768 和 ML-DSA-65 保护数据安全
  - icon: 🌐
    title: P2P 网络
    details: 基于 Kademlia DHT 的去中心化节点发现
  - icon: 💰
    title: 积分激励
    details: 贡献存储和带宽获得 LDC 积分奖励
  - icon: 🚀
    title: 高可用
    details: Reed-Solomon 纠删码确保数据冗余
---

## 定价

- **存储**: 0.5 credit/GB
- **流量**: 0.001 credit/MB
- **充值**: 1 LDC = 1 credit

## 快速体验

```bash
# 下载节点程序
curl -fsSL https://ldrive.io/install.sh | sh

# 启动存储节点
ldrive-node storage --quota 50GB

# 上传文件
ldrive-node publish myfile.zip
```
