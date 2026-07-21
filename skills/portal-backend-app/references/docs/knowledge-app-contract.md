# `knowledge` App — Platform Contract (resource server → portal)

> **Deliverable for the platform team** (portal-integration.md §8, Phase 0/3).
> Everything here is **finalized against the implemented code** — the
> route→scope→PID table below mirrors `src/http/gate.zig` (named metas) +
> `src/bin/xgent_server.zig::routeMetaFor` 1:1, and a regression test
> (`routeMetaFor: every business /v1 route has scope/PID …`) keeps them in sync.
>
> Source of truth: `src/core/auth.zig` (Claims/gates contract),
> `src/http/introspect.zig` (introspection), `src/http/gate.zig` (gates +
> table), `src/storage/visibility.zig` (DataScope mapping).

## 1. Listing metadata

| field | value |
| --- | --- |
| `listingKey` | `knowledge` |
| `version` | `1.0.0` |
| `aud` (accepted TDT audience) | `knowledge` (= listingKey; override `KNOWLEDGE_AUDIENCE`) |
| `scopes` | `knowledge.read`, `knowledge.write` (+ `audit.write` for the audit writer; `files.*` is **future version** only) |
| `dependencies` | `["files"]` — placeholder kept for install topology; **not called at runtime this iteration** (D5) |
| `embedUrl` | _(no embedded micro-app this iteration — pure backend; platform sets when a frontend lands)_ |
| `allowedOrigins` | `PORTAL_BASE_URL` (portal web origin) |
| integration style | **#3 independent backend** (own process + own Postgres/Chroma; does not hold the portal HS256 key, does not verify JWT signatures) |

### navItems (platform `manifests.ts`)

```jsonc
[
  { "id": "nav-kb",    "path": "/kb",    "icon": "book",      "label": { "zh-CN": "知识库", "en": "KB" } },
  { "id": "nav-mem",   "path": "/mem",   "icon": "brain",     "label": { "zh-CN": "记忆", "en": "Memory" } },
  { "id": "nav-code",  "path": "/code",  "icon": "code",      "label": { "zh-CN": "代码检索", "en": "Code" } },
  { "id": "nav-graph", "path": "/graph", "icon": "git-graph", "label": { "zh-CN": "知识图谱", "en": "Graph" } }
]
```

## 2. Runtime contract (already implemented in this server)

1. `Authorization: Bearer <TDT>` missing → **401 `UNAUTHENTICATED`**.
2. `POST {PORTAL_INTROSPECT_URL}`, header `Authorization: Basic base64(saClientId:saSecret)`,
   body `{"token":"<TDT>"}`. `active:false` is a **success** (not a transport error).
   Result cached to `min(exp, now+60s)`, keyed on `sha256(token)`.
3. **Four gates** (central, in `dispatch()` before any handler):
   - aud: `claims.aud == "knowledge"` else **401 `INVALID_TOKEN`**.
   - scope: required scope ∈ `claims.scopes` else **403 `INSUFFICIENT_SCOPE`**.
   - role: structural management requires `claims.role == "admin"` else **403 `FORBIDDEN`**.
   - perm: `claims.bypass || claims.permissions` matches PID else **403 `INSUFFICIENT_PERMISSION`**.
4. **service-kind TDT** (`kind:"service"`, no `user_id`): scope-only; rejected on
   user-owned writes (mem/ctx writes) with **403 `INSUFFICIENT_PERMISSION`**.
5. **Tenant isolation**: every query is scoped by `claims.tenant_id`; request-body
   `tenantId` is never trusted.
6. **Rate limit**: per `(aud, tenant)` fixed 60s window, default 600/min →
   **429 `RATE_LIMITED`**.
7. **Introspection response shape consumed**: `{ active, kind, aud, tenant_id,
   user_id, scopes[], role, bypass, groups[], permissions:[{pid,scope}], exp }`
   (`aclStamp` accepted + ignored).

> PID syntax `knowledge:<page|action>:<key>`; trailing-`*` wildcard, coarse→fine:
> `knowledge:*` ⊃ `knowledge:page:*` ⊃ `knowledge:page:docs.*` ⊃ exact.
> DataScope `own` / `all` only (no `team` this iteration — D4; a `team` grant
> degrades to `own`, the narrowest).

## 3. Route → scope → PID table (= `gate.zig` + `routeMetaFor`)

| Route | method | scope | PID | extra gate |
| --- | --- | --- | --- | --- |
| `/v1/kb/query` | POST | `knowledge.read` | `knowledge:page:kb` | |
| `/v1/kb/add`, `/v1/ingest` | POST | `knowledge.write` | `knowledge:action:kb.add` | |
| `/v1/mem/recall`, `/v1/mem/wake_up` | POST | `knowledge.read` | `knowledge:page:mem` | |
| `/v1/mem/fact`, `/v1/mem/note`, `/v1/mem/diary_save` | POST | `knowledge.write` | `knowledge:action:mem.write` | require user ctx |
| `/v1/mem/invalidate` | POST | `knowledge.write` | `knowledge:action:mem.invalidate` | require user ctx |
| `/v1/code/<read tool>` | POST | `knowledge.read` | `knowledge:page:code` | |
| `/v1/code/code.edit`,`code.snapshot`,`code.index` | POST | `knowledge.write` | `knowledge:action:code.edit` | |
| `/v1/ctx/assemble`, `…/{id}/context` | POST/GET | `knowledge.read` | `knowledge:page:ctx` | |
| `/v1/ctx/sessions`, `…/{id}/messages`, `/v1/ctx/skills` | POST | `knowledge.write` | `knowledge:page:ctx` | require user ctx |
| `/v1/ctx/sessions/{id}/commit` | POST | `knowledge.write` | `knowledge:action:ctx.commit` | require user ctx |
| `/v1/graph/<read>` (query/neighbors/diff_view/lineage/snapshot_at/meta/nl_query/detect_bridges/group_wisdom) | POST | `knowledge.read` | `knowledge:page:graph` | |
| `/v1/graph/<write>` (assert_relation/project/snapshot/log/commit_diff/align/assert_tension/resolve_tension/recompute_decay/cluster_recompute/mark_bridge) | POST | `knowledge.write` | `knowledge:action:graph.write` | |
| `/v1/graph/branch`,`merge`,`revert`,`set_acl` | POST | `knowledge.write` | `knowledge:action:graph.history` | **require admin** |
| `/v1/admin/*` | POST | — | — | `xa-` break-glass (non-portal, D2) |
| `/health`, `/v1/healthz`, `/v1/metrics` | GET | — | — | public |

> New `/v1/graph/<tool>` routes default to `graph_write` (fail-safe — never
> ungated). `set_acl` is admin-gated but **not shipped** this iteration (D6).

**Request / response bodies (for the frontend):** this table is route→scope→PID
only — it does **not** carry field-level request/response shapes. Those are the
per-pillar Zig `*Args` / `*Result` structs in `src/{kb,mem,code,ctx,graph}/api.zig`;
`docs/api/*.md` is a generated one-line tool catalog over the same structs (not
JSON schemas). All responses use the unified envelope `{ ok, data | error }`
([SSO指引 §8.1]). HTTP route → handler dispatch is in `src/bin/xgent_server.zig`
(grep `"/v1/<pillar>/"`); route names and tool names do **not** map 1:1, so read
the dispatch + `*Args` struct for each route. _If the frontend needs concrete
per-route request/response examples, ask the knowledge team for a generated API
reference — it is not part of this contract yet._

## 4. ACL Manifest JSON (platform `manifests.ts`)

```jsonc
{
  "version": "1.0.0",
  "landingPageKey": "kb",
  "groups": [{ "key": "kb", "label": { "zh-CN": "知识库", "en": "Knowledge" }, "sort": 0 }],
  "pages": [
    { "key": "kb",    "path": "/kb",    "label": { "zh-CN": "知识库", "en": "KB" },      "navItemId": "nav-kb",    "parentKey": "kb", "supportedScopes": ["own", "all"], "defaultForMember": true },
    { "key": "mem",   "path": "/mem",   "label": { "zh-CN": "记忆", "en": "Memory" },    "navItemId": "nav-mem",   "parentKey": "kb", "supportedScopes": ["own", "all"], "defaultForMember": true },
    { "key": "code",  "path": "/code",  "label": { "zh-CN": "代码检索", "en": "Code" },  "navItemId": "nav-code",  "parentKey": "kb", "supportedScopes": ["all"] },
    { "key": "ctx",   "path": "/ctx",   "label": { "zh-CN": "上下文", "en": "Context" },                          "parentKey": "kb", "supportedScopes": ["own", "all"] },
    { "key": "graph", "path": "/graph", "label": { "zh-CN": "知识图谱", "en": "Graph" }, "navItemId": "nav-graph", "parentKey": "kb", "supportedScopes": ["own", "all"] },
    { "key": "admin", "path": "/admin", "label": { "zh-CN": "知识库管理" }, "supportedScopes": ["all"] }
  ],
  "actions": [
    { "key": "kb.add",         "label": { "zh-CN": "导入文档/Ingest" }, "pageKey": "kb",   "supportedScopes": ["own", "all"], "defaultForMember": true },
    { "key": "mem.write",      "label": { "zh-CN": "写入记忆" },         "pageKey": "mem",  "supportedScopes": ["own"], "defaultForMember": true },
    { "key": "mem.invalidate", "label": { "zh-CN": "失效记忆" },         "pageKey": "mem",  "dangerous": true, "supportedScopes": ["own"] },
    { "key": "code.edit",      "label": { "zh-CN": "编辑代码" },         "pageKey": "code", "dangerous": true, "supportedScopes": ["all"] },
    { "key": "ctx.commit",     "label": { "zh-CN": "提交会话" },         "pageKey": "ctx",  "supportedScopes": ["own"], "defaultForMember": true },
    { "key": "graph.write",    "label": { "zh-CN": "写图谱（断言/项目化/快照）" }, "pageKey": "graph", "supportedScopes": ["own", "all"], "defaultForMember": true },
    { "key": "graph.history",  "label": { "zh-CN": "图谱版本（分支/合并/回退）" }, "pageKey": "graph", "dangerous": true, "supportedScopes": ["all"] }
  ],
  "roleTemplates": [
    { "key": "kb-viewer", "name": { "zh-CN": "知识库·只读" }, "grants": [{ "pid": "knowledge:page:*" }] },
    { "key": "kb-editor", "name": { "zh-CN": "知识库·读写" },
      "grants": [{ "pid": "knowledge:page:*" }, { "pid": "knowledge:action:kb.add" },
                 { "pid": "knowledge:action:mem.write" }, { "pid": "knowledge:action:ctx.commit" },
                 { "pid": "knowledge:action:graph.write" }] },
    { "key": "kb-admin",  "name": { "zh-CN": "知识库·管理" }, "grants": [{ "pid": "knowledge:*" }] }
  ]
}
```

## 5. Env contract (Appendix B; image contains NO `.env`)

| var | required | default / example | notes |
| --- | --- | --- | --- |
| `XGENT_PG_DSN` | ✅ | `postgres://…/xgent_kb` | this server's own Postgres (portal-contract alias: `KNOWLEDGE_DATABASE_URL`) |
| `XGENT_CHROMA_URL` | ✅ | `http://127.0.0.1:8000` | vector store |
| `XGENT_OPENAI_KEY` | ✅ | `sk-…` | embedding key |
| `PORTAL_INTROSPECT_URL` | ✅* | `http://localhost:3000/api/tokens/introspect` | *required to enable portal auth; partial config → fail-fast |
| `KNOWLEDGE_SA_CLIENT_ID` | ✅* | `knowledge-server` | service-account Basic id |
| `KNOWLEDGE_SA_CLIENT_SECRET` | ✅* | — | service-account Basic secret |
| `KNOWLEDGE_AUDIENCE` | | `knowledge` | accepted TDT `aud` |
| `KNOWLEDGE_SERVER_PORT` | | `8080` (image default) | container/prod listen port — the reverse proxy routes to the fixed `knowledge-server:8080`. Local bare-run may set 4700. Override order: this → `XGENT_HTTP_PORT` → `--http-port`. **No bare `PORT` var is read.** |
| `API_BASE_URL` | | `http://localhost:3000` | portal Open API base (audit) |
| `PORTAL_BASE_URL` | | `http://localhost:5300` | portal web origin (CORS) |
| `KNOWLEDGE_RATE_LIMIT_PER_MIN` | | `600` | inbound `(aud,tenant)` limit |
| `REDIS_CONN_STRING` | | — | optional; unset → in-process cache + limiter |
| `XGENT_ADMIN_HMAC_KEYS` | | — | `xa-` break-glass (non-portal, D2) |

\* The three portal vars are an all-or-nothing set: none → portal auth disabled
(gated routes 503 `AUTH_UNAVAILABLE`, only `xa-` works); any-but-not-all →
**server refuses to start** and prints the missing var.

## 6. Platform-team checklist (the external blockers — §7)

1. Register scopes `knowledge.read` / `knowledge.write` in `packages/shared/src/scopes.ts`.
2. Register the ACL Manifest (§4) in `apps/api/src/modules/acl/manifests.ts`.
3. Fill the `knowledge` listing: `scopes`, `navItems`, `dependencies:["files"]`, stamp `aclManifest`.
4. `SERVICE_REGISTRY` + reverse proxy `/svc/knowledge/` (+ Caddy), CORS allowlist, CSP `frame-src`/`connect-src`.
5. **Create the service account** (capability `token.introspect`) → send `clientId`/`secret` to this repo. **(Phase 1 blocker.)**
6. _(future version)_ `knowledge→files` token-exchange grant + files allowlist (not needed this iteration — D5).
7. Point compose profile / K8s Deployment at this repo's **external standalone image** (`Dockerfile`), not the monorepo image.
8. Deliver the local devkit (pre-registered `knowledge` portal image) for §6 Phase 7 end-to-end.

## 7. Ops commands

- DB migrations: `scripts/migrate.sh up` (idempotent; `status` / `down [N]`). The server does **not** auto-migrate.
- Per-tenant bootstrap: every business table FKs `tenants(id)`, and the server uses the TDT's `tenant_id` (the **portal tenant UUID**) verbatim with **no auto-create**. So before a tenant's first write, insert a `tenants` row whose `id` **equals that portal tenant UUID**:
  ```sql
  INSERT INTO tenants (id, slug) VALUES ('<portal-tenant-uuid>', '<slug>')
    ON CONFLICT (id) DO NOTHING;
  ```
  A plain `INSERT INTO tenants (slug) …` mints a fresh `gen_random_uuid()` that will **not** match the TDT → foreign-key violation on first write.
- Liveness/readiness: `GET /health` → `{"service":"knowledge","db":"ok|down","redis":"disabled","time":<unix>}` (200 = ready, 503 = DB down). Liveness-only probe: `GET /v1/healthz` → `{"status":"ok"}`.
- Image delivery: `docker save` / private registry (not public).

## 8. Self-test (no full UI — Appendix C)

```bash
# aud/scope correct → 200; else 401/403
curl -X POST http://127.0.0.1:8080/v1/kb/query \
  -H "Authorization: Bearer <aud=knowledge TDT>" \
  -H "content-type: application/json" \
  -d '{"query":"hello","top_k":5}'
```

## 9. MCP trust boundary (G16 / §5.8 — hard rule)

The `--role mcp` stdio surface has **no bearer, no introspection, no gates** —
it constructs `AuthCtx` directly from tool-call args (`tenant_id`/`user_id`)
and fully trusts them. It is for **local/dev/trusted** processes only and must
**never** be exposed to the portal or any untrusted network. Portal
integration is HTTP + TDT only. The shipped image (`Dockerfile`) runs
`--role http` exclusively and does not map/route the MCP stdio; if an operator
enables MCP they must ensure its stdio is not externally reachable.
