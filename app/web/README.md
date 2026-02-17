# LDrive Web Frontend

SvelteKit frontend for LDrive file upload/download.

## Deploy to Cloudflare Pages

```bash
npm install
npm run build
npx wrangler pages deploy .svelte-kit/cloudflare
```

Or connect your Git repo to Cloudflare Pages dashboard:
- Build command: `npm run build`
- Build output directory: `.svelte-kit/cloudflare`

## Deploy to Vercel

```bash
npm install
npx vercel
```

## Local Development

```bash
npm install
npm run dev
```
