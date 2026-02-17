import { Hono } from 'hono';
import { serve } from '@hono/node-server';
import fileRoutes from './routes/file.js';
import authRoutes from './auth.js';

const app = new Hono();

app.get('/', (c) => c.text('LDrive API Server'));
app.route('/api', fileRoutes);
app.route('/api/auth', authRoutes);

const port = 3000;
console.log(`Server running on http://localhost:${port}`);

serve({
  fetch: app.fetch,
  port,
});
