import { Hono } from 'hono';

const app = new Hono();

// LinuxDO OAuth2 endpoints
const OAUTH_AUTHORIZE = 'https://connect.linux.do/oauth2/authorize';
const OAUTH_TOKEN = 'https://connect.linux.do/oauth2/token';

app.get('/login', (c) => {
  const clientId = process.env.LINUXDO_CLIENT_ID || '';
  const redirectUri = process.env.LINUXDO_REDIRECT_URI || 'http://localhost:3000/api/auth/callback';

  const authUrl = `${OAUTH_AUTHORIZE}?client_id=${clientId}&redirect_uri=${encodeURIComponent(redirectUri)}&response_type=code&scope=read`;

  return c.redirect(authUrl);
});

app.get('/callback', async (c) => {
  const code = c.req.query('code');

  if (!code) {
    return c.text('Missing authorization code', 400);
  }

  try {
    const tokenResp = await fetch(OAUTH_TOKEN, {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        code,
        client_id: process.env.LINUXDO_CLIENT_ID || '',
        client_secret: process.env.LINUXDO_CLIENT_SECRET || '',
        redirect_uri: process.env.LINUXDO_REDIRECT_URI || 'http://localhost:3000/api/auth/callback',
      }),
    });

    const tokenData = await tokenResp.json();
    const sessionToken = Buffer.from(JSON.stringify({
      access_token: tokenData.access_token,
      expires_at: Date.now() + 3600000
    })).toString('base64');

    return c.json({ token: sessionToken, message: 'Login success' });
  } catch (e) {
    return c.text('Token exchange failed', 500);
  }
});

export default app;
