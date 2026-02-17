# Cloudflare Pages 配置

## 自动部署设置

1. 访问 [Cloudflare Pages Dashboard](https://dash.cloudflare.com/pages)
2. 点击 "Create a project" → "Connect to Git"
3. 选择 GitHub 仓库：`rand0mdevel0per/ldrive`
4. 配置构建设置：
   - **Framework preset**: SvelteKit
   - **Build command**: `cd app/web && npm install && npm run build`
   - **Build output directory**: `app/web/.svelte-kit/cloudflare`
   - **Root directory**: `/` (留空或选择根目录)

5. 点击 "Save and Deploy"

## 环境变量（可选）

如需配置环境变量，在 Pages 项目设置中添加：
- `PUBLIC_API_URL`: API 服务器地址
- `PUBLIC_GATEWAY_URL`: Gateway 地址

## 自动部署

配置完成后，每次推送到 `master` 分支都会自动触发部署。

## 手动部署（备选）

```bash
cd app/web
npm install
npm run build
npx wrangler pages deploy .svelte-kit/cloudflare --project-name=ldrive-web
```
