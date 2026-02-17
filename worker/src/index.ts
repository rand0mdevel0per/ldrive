import { Hono } from 'hono';
import { basicAuth } from 'hono/basic-auth';
import { createPayment } from './ldc.js';
import webdavRoutes from './webdav.js';

type Bindings = {
  GATEWAY_URL: string;
  LDC_PID: string;
  LDC_SECRET: string;
  WEBDAV_USER: string;
  WEBDAV_PASS: string;
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

app.post('/pay', async (c) => {
  const { fileSize, fileName } = await c.req.json();
  const money = Math.ceil(fileSize / (1024 * 1024 * 1024)) * 0.1;
  const outTradeNo = `LD${Date.now()}`;

  try {
    const payUrl = await createPayment(
      c.env.LDC_PID,
      c.env.LDC_SECRET,
      money,
      `LDrive存储: ${fileName}`,
      outTradeNo
    );
    return c.json({ payUrl, outTradeNo });
  } catch (e) {
    return c.json({ error: 'Payment failed' }, 500);
  }
});

app.get('/file/:hash', async (c) => {
  const hash = c.req.param('hash');
  const gatewayUrl = c.env.GATEWAY_URL;

  try {
    const resp = await fetch(`${gatewayUrl}/api/file/${hash}`);
    if (!resp.ok) {
      return c.text('File not found', 404);
    }

    return new Response(resp.body, {
      headers: {
        'Content-Type': resp.headers.get('Content-Type') || 'application/octet-stream',
        'Content-Disposition': resp.headers.get('Content-Disposition') || 'attachment',
      },
    });
  } catch (e) {
    return c.text('Gateway error', 502);
  }
});

export default app;
