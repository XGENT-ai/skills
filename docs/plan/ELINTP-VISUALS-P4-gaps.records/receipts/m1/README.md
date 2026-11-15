# M1 验收回执（V-2 三条命令）

- 采集时间：2026-11-15 13:08 CST
- 环境：mise 固定 rustc 1.99.1 + mbx（`--locked --offline`）、Node 25.2.1
- 每条命令独立执行，退出码单独保存（不是复合命令里的 echo）：

| 命令 | 退出码 | 原始输出 |
| --- | --- | --- |
| `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test chart` | 0（`chart-test.exit`） | `chart-test.log`：`test result: ok. 16 passed; 0 failed` |
| `mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check` | 0（`assemble-check.exit`） | `assemble-check.log`：`template.html` sha256 `688a1811…`、`catalogs.json` sha256 `53b662eb…` |
| `mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/catalogs.test.mjs` | 0（`catalogs-test.exit`） | `catalogs-test.log`：`tests 9 / pass 9 / fail 0` |

同日又以宿主可观测的独立命令重跑上述三条命令，三条均 exit 0，输出与上表一致（16 passed / --check 同组 sha256 / 9 passed）。
