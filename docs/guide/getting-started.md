# 快速开始

## 安装

### 下载二进制

```bash
# Linux/macOS
curl -fsSL https://ldrive.io/install.sh | sh

# Windows
# 从 GitHub Releases 下载
```

### 从源码编译

```bash
git clone https://github.com/rand0mdevel0per/ldrive
cd ldrive
cargo build --release
```

## 运行节点

### 存储节点

```bash
ldrive-node storage \
  --storage-path ./data \
  --quota 100GB \
  --bootstrap gateway.ldrive.io:4433
```

### 网关节点

```bash
ldrive-node gateway \
  --public-addr YOUR_IP:4433 \
  --relay-enabled
```

## 上传文件

```bash
ldrive-node publish myfile.zip
# 输出: file_hash = abc123...
```

## 下载文件

```bash
ldrive-node fetch abc123... output.zip
```

## Web 界面

访问 https://ldrive-web.pages.dev 使用 Web 界面上传下载文件。
