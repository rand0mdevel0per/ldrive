// 积分定价配置
export const PRICING = {
  LDC_TO_CREDIT: 1,           // 1 LDC = 1 credit
  STORAGE_PER_GB: 0.5,        // 0.5 credit/GB
  BANDWIDTH_PER_MB: 0.001,    // 0.001 credit/MB
  PLATFORM_FEE: 0.05,         // 5% 平台抽成
};

// 计算存储费用
export function calculateStorageCost(sizeBytes: number): number {
  const gb = sizeBytes / (1024 * 1024 * 1024);
  return Math.ceil(gb * PRICING.STORAGE_PER_GB * 100) / 100;
}

// 计算流量费用
export function calculateBandwidthCost(sizeBytes: number): number {
  const mb = sizeBytes / (1024 * 1024);
  return Math.ceil(mb * PRICING.BANDWIDTH_PER_MB * 1000) / 1000;
}

// LDC 转积分
export function ldcToCredits(ldc: number): number {
  return ldc * PRICING.LDC_TO_CREDIT;
}

// 积分转 LDC
export function creditsToLdc(credits: number): number {
  return credits / PRICING.LDC_TO_CREDIT;
}
