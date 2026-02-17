import { Hono } from 'hono';

type Bindings = {
  GATEWAY_URL: string;
};

const app = new Hono<{ Bindings: Bindings }>();

app.on('OPTIONS', '/*', (c) => {
  return c.body('', 200, {
    'DAV': '1, 2',
    'Allow': 'OPTIONS, GET, PUT, PROPFIND, DELETE',
  });
});

app.on('PROPFIND', '/', (c) => {
  const xml = `<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>`;
  return c.body(xml, 207, { 'Content-Type': 'application/xml' });
});

app.on('PROPFIND', '/files/*', (c) => {
  const xml = `<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/files/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>`;
  return c.body(xml, 207, { 'Content-Type': 'application/xml' });
});

app.put('/files/:filename', async (c) => {
  const filename = c.req.param('filename');
  const gatewayUrl = c.env.GATEWAY_URL;

  try {
    const formData = new FormData();
    formData.append('file', await c.req.blob(), filename);

    const resp = await fetch(`${gatewayUrl}/api/upload`, {
      method: 'POST',
      body: formData,
    });

    if (!resp.ok) {
      return c.text('Upload failed', 500);
    }

    return c.text('', 201);
  } catch (e) {
    return c.text('Error', 500);
  }
});

app.get('/files/:hash', async (c) => {
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
      },
    });
  } catch (e) {
    return c.text('Error', 500);
  }
});

export default app;
