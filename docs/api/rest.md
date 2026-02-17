# REST API

## Gateway API

### 充值

```http
POST /recharge
Content-Type: application/json

{
  "amount": 10
}
```

返回：
```json
{
  "payUrl": "https://credit.linux.do/epay/...",
  "outTradeNo": "LDR1234567890",
  "credits": 10
}
```

### 上传支付

```http
POST /pay
Content-Type: application/json

{
  "fileSize": 1073741824,
  "fileName": "myfile.zip"
}
```

返回：
```json
{
  "payUrl": "https://credit.linux.do/epay/...",
  "outTradeNo": "LD1234567890",
  "credits": 0.5,
  "ldc": 0.5
}
```

### 下载文件

```http
GET /file/:hash
```

返回文件流。

## Node API

### 发布文件

```bash
ldrive-node publish <file_path>
```

### 获取文件

```bash
ldrive-node fetch <file_hash> <output_path>
```
