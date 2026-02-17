// 用户余额管理
export interface UserBalance {
  userId: string;
  credits: number;
  lastUpdated: number;
}

// 检查余额是否足够
export function hasEnoughBalance(balance: number, required: number): boolean {
  return balance >= required;
}

// 扣除余额
export function deductBalance(balance: number, amount: number): number {
  return Math.max(0, balance - amount);
}

// 增加余额
export function addBalance(balance: number, amount: number): number {
  return balance + amount;
}

// 从 KV 获取余额
export async function getBalance(kv: KVNamespace, userId: string): Promise<number> {
  const data = await kv.get(`balance:${userId}`);
  return data ? parseFloat(data) : 0;
}

// 保存余额到 KV
export async function setBalance(kv: KVNamespace, userId: string, balance: number): Promise<void> {
  await kv.put(`balance:${userId}`, balance.toString());
}
