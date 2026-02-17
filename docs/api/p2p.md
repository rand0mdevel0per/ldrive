# P2P 协议

## 传输层

- **QUIC** (UDP): 优先使用
- **WSS** (TCP 443): QUIC 被 QoS 时降级
- **TURN Relay**: NAT 穿透失败时中继

## DHT (Kademlia)

- 160-bit 节点 ID
- K = 20 (K-Bucket 容量)
- Alpha = 3 (并行查找)

### 消息类型

- `PING/PONG`: 心跳
- `FIND_NODE`: 查找节点
- `FIND_VALUE`: 查找内容
- `STORE`: 存储内容映射

## 分块与纠删

- **FastCDC**: 64KB 均值分块
- **Reed-Solomon 4+2**: 容忍 2 节点故障
- **BLAKE3**: 内容哈希

## Vnode 分布

- 每个物理节点 128 个虚拟节点
- 3+3 分片策略：3 块同区域，3 块异区域
