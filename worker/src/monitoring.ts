// 监控指标
export interface Metrics {
  uploads: number;
  downloads: number;
  bandwidth: number;
  errors: number;
  timestamp: number;
}

// 记录指标
export async function recordMetric(
  kv: KVNamespace,
  type: 'upload' | 'download' | 'error',
  size: number = 0
): Promise<void> {
  const key = `metrics:${new Date().toISOString().split('T')[0]}`;
  const data = await kv.get(key);
  const metrics: Metrics = data ? JSON.parse(data) : {
    uploads: 0,
    downloads: 0,
    bandwidth: 0,
    errors: 0,
    timestamp: Date.now(),
  };

  if (type === 'upload') metrics.uploads++;
  if (type === 'download') {
    metrics.downloads++;
    metrics.bandwidth += size;
  }
  if (type === 'error') metrics.errors++;

  await kv.put(key, JSON.stringify(metrics), { expirationTtl: 86400 * 30 });
}

// 获取指标
export async function getMetrics(kv: KVNamespace, date?: string): Promise<Metrics | null> {
  const key = `metrics:${date || new Date().toISOString().split('T')[0]}`;
  const data = await kv.get(key);
  return data ? JSON.parse(data) : null;
}
