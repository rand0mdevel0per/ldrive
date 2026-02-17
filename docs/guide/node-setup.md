# 节点部署

## 节点类型

### Storage Node (存储节点)
- 贡献存储空间
- 内网设备即可
- 赚取存储积分

### Gateway Node (网关节点)
- 需要公网 IP
- 提供 TURN 中继
- 赚取带宽积分

### Hybrid Node (混合节点)
- 存储 + 网关
- 最大化收益

## 配置示例

```bash
# 存储节点
ldrive-node storage \
  --storage-path /data/ldrive \
  --quota 50GB \
  --bootstrap gateway1.ldrive.io:4433

# 网关节点
ldrive-node gateway \
  --public-addr 1.2.3.4:4433 \
  --relay-enabled

# 混合节点
ldrive-node hybrid \
  --storage-path /data/ldrive \
  --quota 100GB \
  --public-addr 1.2.3.4:4433
```

## 系统要求

- CPU: 1 核+
- 内存: 512MB+
- 存储: 根据配额
- 网络: 稳定连接

## 区域亲和性

节点自动检测区域（通过 ip.sb），优先与同区域节点通信。
