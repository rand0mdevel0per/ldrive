import { Context, Next } from 'hono';
import { getBalance } from './balance.js';

type Env = {
  CREDITS_KV: KVNamespace;
};

// 余额检查中间件
export async function requireBalance(minBalance: number = 0) {
  return async (c: Context<{ Bindings: Env }>, next: Next) => {
    const userId = c.req.header('X-User-ID') || 'anonymous';
    const balance = await getBalance(c.env.CREDITS_KV, userId);

    if (balance < minBalance) {
      return c.json({ error: 'Insufficient balance', balance, required: minBalance }, 403);
    }

    c.set('userId', userId);
    c.set('balance', balance);
    await next();
  };
}

// 速率限制中间件
export async function rateLimit(maxRequests: number, windowMs: number) {
  return async (c: Context<{ Bindings: Env }>, next: Next) => {
    const userId = c.req.header('X-User-ID') || c.req.header('CF-Connecting-IP') || 'anonymous';
    const key = `ratelimit:${userId}:${Date.now() / windowMs | 0}`;

    const count = await c.env.CREDITS_KV.get(key);
    const current = count ? parseInt(count) : 0;

    if (current >= maxRequests) {
      return c.json({ error: 'Rate limit exceeded' }, 429);
    }

    await c.env.CREDITS_KV.put(key, (current + 1).toString(), { expirationTtl: windowMs / 1000 });
    await next();
  };
}
