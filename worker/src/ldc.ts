const LDC_BASE = 'https://credit.linux.do/epay';

async function md5(text: string): Promise<string> {
  const encoder = new TextEncoder();
  const data = encoder.encode(text);
  const hashBuffer = await crypto.subtle.digest('MD5', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('');
}

function buildSign(params: Record<string, string>, secret: string): string {
  const sorted = Object.keys(params)
    .filter(k => params[k] && k !== 'sign' && k !== 'sign_type')
    .sort()
    .map(k => `${k}=${params[k]}`)
    .join('&');
  return sorted + secret;
}

export async function createPayment(
  pid: string,
  secret: string,
  money: number,
  name: string,
  outTradeNo: string
): Promise<string> {
  const params: Record<string, string> = {
    pid,
    type: 'epay',
    out_trade_no: outTradeNo,
    name,
    money: money.toFixed(2),
  };

  const signStr = buildSign(params, secret);
  const sign = await md5(signStr);

  const form = new URLSearchParams({ ...params, sign, sign_type: 'MD5' });
  const resp = await fetch(`${LDC_BASE}/pay/submit.php`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: form,
    redirect: 'manual',
  });

  if (resp.status === 302) {
    return resp.headers.get('Location') || '';
  }

  throw new Error('Payment creation failed');
}
