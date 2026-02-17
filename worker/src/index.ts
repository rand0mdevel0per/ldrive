import { Hono } from 'hono';
import { basicAuth } from 'hono/basic-auth';
import { createPayment } from './ldc.js';
import webdavRoutes from './webdav.js';
import { calculateStorageCost, calculateBandwidthCost, creditsToLdc } from './pricing.js';
import { requireBalance, rateLimit } from './middleware.js';
import { recordMetric, getMetrics } from './monitoring.js';

type Bindings = {
  GATEWAY_URL: string;
  LDC_PID: string;
  LDC_SECRET: string;
  WEBDAV_USER: string;
  WEBDAV_PASS: string;
  CREDITS_KV: KVNamespace;
  LD_CLIENT_ID: string;
  LD_CLIENT_SECRET: string;
};

const app = new Hono<{ Bindings: Bindings }>();

app.get('/', (c) => c.text('LDrive Gateway'));

app.use('/dav/*', async (c, next) => {
  const auth = basicAuth({
    username: c.env.WEBDAV_USER,
    password: c.env.WEBDAV_PASS,
  });
  return auth(c, next);
});

app.route('/dav', webdavRoutes);

app.post('/pay', rateLimit(10, 60000), async (c) => {
  const { fileSize, fileName } = await c.req.json();
  const credits = calculateStorageCost(fileSize);
  const ldc = creditsToLdc(credits);
  const outTradeNo = `LD${Date.now()}`;

  try {
    const payUrl = await createPayment(
      c.env.LDC_PID,
      c.env.LDC_SECRET,
      ldc,
      `LDrive存储: ${fileName} (${credits} credits)`,
      outTradeNo
    );
    return c.json({ payUrl, outTradeNo, credits, ldc });
  } catch (e) {
    return c.json({ error: 'Payment failed' }, 500);
  }
});

app.post('/recharge', async (c) => {
  const { amount } = await c.req.json();
  const outTradeNo = `LDR${Date.now()}`;

  try {
    const payUrl = await createPayment(
      c.env.LDC_PID,
      c.env.LDC_SECRET,
      amount,
      `LDrive充值: ${amount} LDC`,
      outTradeNo
    );
    return c.json({ payUrl, outTradeNo, credits: amount });
  } catch (e) {
    return c.json({ error: 'Recharge failed' }, 500);
  }
});

app.get('/file/:hash', requireBalance(0.001), rateLimit(20, 60000), async (c) => {
  const hash = c.req.param('hash');
  const gatewayUrl = c.env.GATEWAY_URL;

  try {
    const resp = await fetch(`${gatewayUrl}/api/file/${hash}`);
    if (!resp.ok) {
      await recordMetric(c.env.CREDITS_KV, 'error');
      return c.text('File not found', 404);
    }

    const size = parseInt(resp.headers.get('Content-Length') || '0');
    await recordMetric(c.env.CREDITS_KV, 'download', size);

    return new Response(resp.body, {
      headers: {
        'Content-Type': resp.headers.get('Content-Type') || 'application/octet-stream',
        'Content-Disposition': resp.headers.get('Content-Disposition') || 'attachment',
      },
    });
  } catch (e) {
    await recordMetric(c.env.CREDITS_KV, 'error');
    return c.text('Gateway error', 502);
  }
});

app.get('/admin/metrics', async (c) => {
  const date = c.req.query('date');
  const metrics = await getMetrics(c.env.CREDITS_KV, date);
  return c.json(metrics || { message: 'No data' });
});

app.post('/oauth/token', async (c) => {
  const { code, redirect_uri } = await c.req.json();

  try {
    const res = await fetch('https://connect.linux.do/oauth2/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        redirect_uri,
        client_id: c.env.LD_CLIENT_ID,
        client_secret: c.env.LD_CLIENT_SECRET,
      }),
    });

    const data = await res.json();
    return c.json(data);
  } catch (e) {
    return c.json({ error: 'OAuth failed' }, 500);
  }
});

app.get('/oauth/user', async (c) => {
  const token = c.req.header('Authorization')?.replace('Bearer ', '');
  if (!token) return c.json({ error: 'No token' }, 401);

  try {
    const res = await fetch('https://connect.linux.do/api/user', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const user = await res.json();
    return c.json(user);
  } catch (e) {
    return c.json({ error: 'Failed to fetch user' }, 500);
  }
});

app.get('/balance/:userId', async (c) => {
  const userId = c.req.param('userId');
  const balance = await c.env.CREDITS_KV.get(`balance:${userId}`);
  return c.json({ balance: balance ? parseFloat(balance) : 0 });
});

app.post('/ldc/notify', async (c) => {
  const data = await c.req.json();
  // TODO: Verify signature and update user balance
  return c.json({ success: true });
});

export default app;
