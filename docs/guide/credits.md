# 积分系统

## 定价

| 项目 | 价格 |
|------|------|
| 存储 | 0.5 credit/GB |
| 流量 | 0.001 credit/MB |
| 充值 | 1 LDC = 1 credit |

## 充值

访问 Gateway 充值页面：

```bash
curl -X POST https://gateway.ldrive.io/recharge \
  -H "Content-Type: application/json" \
  -d '{"amount": 10}'
```

返回支付链接，完成 LDC 支付即可获得积分。

## 消费

- **上传文件**: 根据文件大小自动扣除存储费用
- **下载文件**: 根据流量大小扣除带宽费用

## 赚取积分

运行存储节点贡献空间和带宽：

```bash
ldrive-node storage --quota 100GB
```

每日结算积分：
- 存储贡献: `ln(1 + GB) × uptime × challenge_pass_rate`
- 带宽贡献: `GB_served × 0.001`

平台抽成 5%。
