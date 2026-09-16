// 独立运行，无需门户源码或凭证；判据与平台运行时同源。
// bun verify-health.ts <health-url> <listingKey> [ready|not-ready]
import { healthVerdict, isHealthy } from "./health-verdict";

const args = Bun.argv.slice(2);
const [url, listingKey, expected = "ready"] = args;
if (args.length < 2 || args.length > 3 || !url || !listingKey || !["ready", "not-ready"].includes(expected)) {
  console.error("用法：bun verify-health.ts <health-url> <listingKey> [ready|not-ready]");
  process.exit(2);
}

try {
  // 与 deploy-controller / health:deployed 相同：5 秒超时，JSON 解析失败按 null 判定。
  const res = await fetch(url, { signal: AbortSignal.timeout(5000) })
    .catch(() => { throw new Error("健康接口无法连接或探测超时"); });
  const body = await res.json().catch(() => null);
  const verdict = healthVerdict(res, body);
  const up = isHealthy(verdict);
  if (up !== (expected === "ready"))
    throw new Error(`期望 ${expected}，实际 ${verdict.verdict}${verdict.detail ? ` (${verdict.detail})` : ""}`);
  console.log(`[health-contract] PASS ${listingKey}: ${verdict.verdict}, HTTP ${res.status}`);
} catch (error) {
  // not-ready 只验证收到了明确的失败响应；连接失败不算故障分支已正确实现。
  // 不打印服务响应体，它可能错误地包含连接串或其他秘密。
  console.error(`[health-contract] FAIL: ${error instanceof Error ? error.message : "探测失败"}`);
  process.exit(1);
}
