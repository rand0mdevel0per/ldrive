import { Hono } from 'hono';
import { exec } from 'child_process';
import { promisify } from 'util';
import { writeFile, unlink } from 'fs/promises';
import { join } from 'path';

const execAsync = promisify(exec);
const app = new Hono();

app.get('/file/:hash', async (c) => {
  const hash = c.req.param('hash');

  try {
    const { stdout } = await execAsync(`ldrive-node fetch ${hash} --output ./downloads --bootstrap 127.0.0.1:4433`);
    return c.json({ message: 'File fetched', hash });
  } catch (e) {
    return c.text('File not found', 404);
  }
});

app.post('/upload', async (c) => {
  const body = await c.req.parseBody();
  const file = body['file'];

  if (!file || typeof file === 'string') {
    return c.json({ error: 'No file provided' }, 400);
  }

  const tempPath = join('/tmp', `upload-${Date.now()}`);
  await writeFile(tempPath, await file.arrayBuffer());

  try {
    const { stdout } = await execAsync(
      `ldrive-node publish ${tempPath} --listen 0.0.0.0:0 --storage ./storage --bootstrap 127.0.0.1:4433`
    );

    const hashMatch = stdout.match(/file_hash = ([a-f0-9]+)/);
    const hash = hashMatch ? hashMatch[1] : null;

    await unlink(tempPath);

    return c.json({ hash, message: 'File uploaded' });
  } catch (e) {
    await unlink(tempPath);
    return c.json({ error: 'Upload failed' }, 500);
  }
});

export default app;
