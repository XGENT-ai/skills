// 部署、平台信息页、巡检共用此判据。skill 带同源副本，export-skills.sh 校验逐字一致。
export type HealthVerdictKind = "healthy" | "degraded" | "unhealthy";

/** 先看 HTTP，再按顶层是否含 ok 分流；不按内建/外部 App 区分。
 *  信封必须 ok===true 且 data.db===true；非信封沿用任意 2xx 通过的兼容规则。
 *  service/redis/time 不参与判定，非信封的依赖故障必须由非 2xx 状态表达。 */
export function healthVerdict(
  res: { ok: boolean; status: number },
  body: unknown,
): { verdict: HealthVerdictKind; detail: string | null } {
  if (!res.ok) return { verdict: "unhealthy", detail: `http ${res.status}` };
  const isEnvelope = body !== null && typeof body === "object" && "ok" in (body as object);
  if (!isEnvelope) return { verdict: "healthy", detail: null };
  const b = body as { ok?: unknown; data?: { db?: unknown } };
  if (b.ok !== true) return { verdict: "degraded", detail: "ok=false" };
  if (b.data?.db === true) return { verdict: "healthy", detail: null };
  return { verdict: "degraded", detail: b.data?.db === false ? "db=false" : "db=unknown" };
}

export function isHealthy(v: { verdict: HealthVerdictKind }): boolean {
  return v.verdict === "healthy";
}
