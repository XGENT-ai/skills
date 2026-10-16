# Phoenix Diagram 固定来源

这里保存五类 Rust 图解移植及开发期 Node oracle 使用的确切源码。来源身份、原始位置和选取范围见 [SOURCE.json](SOURCE.json)，许可见 [../NOTICE.md](../NOTICE.md)。

| 来源 | 固定身份 | 交付物 |
| --- | --- | --- |
| 用户提供的本地 3.1.0 快照，未核对上游提交 | 内容 SHA-256 `4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395` | [archify-3.1.0-source.tar.gz](archify-3.1.0-source.tar.gz)、[archify-3.1.0-manifest.json](archify-3.1.0-manifest.json) |
| XGENT-ai/skills 的 legacy-2.17 子集 | Git 提交 `4a65a1525f9ed03c775a0e18935901e1edbb773f` | [legacy-2.17/](legacy-2.17/)、[legacy-2.17-manifest.json](legacy-2.17-manifest.json) |

3.1.0 归档包含 archify、viewer、scripts、test 四个目录和根 package.json、package-lock.json、LICENSE、THIRD_PARTY_NOTICES.md，共 498 个普通文件。USTAR 成员按路径 UTF-8 字节升序，mode 为 0644，mtime、uid、gid 为 0，uname、gname 为空，gzip mtime 为 0。归档 SHA-256 为 `6831ba7f7d9d5c0fd473e317bd9b513f2581b9d4d63bd157d1de4680af3b32a4`。

两份 manifest 都是 `[POSIX 相对路径, 字节数, sha256]` 数组，按路径 UTF-8 字节升序排列，使用 UTF-8、无空白 JSON、无结尾换行。对 manifest 的确切字节计算 SHA-256 即得到内容摘要。legacy 子集共 70 个文件，内容摘要为 `9b02b58082c1872f611fff050b225026e108ca53efa738da429965db9966d273`；保留六份 schema、14 份五类示例、JSON 兼容 fixture、renderer 导入闭包、输出检查器及其必要模板和许可。

从仓库根显式准备维护工具与锁定依赖，然后核对来源和离线构建前置条件：

```sh
node scripts/prepare-phoenix-diagram.mjs --prepare
node scripts/check-phoenix-diagram-source.mjs --check
```

prepare 使用 `build-tools.lock.json` 的 Rust、Node、mr-boxington 版本，默认只在 Cargo.lock 缺失时生成初始锁，然后执行 `mbx fetch --locked`。维护者明确修改 Cargo.toml 后，用 `mise exec --no-deps -- node scripts/prepare-phoenix-diagram.mjs --prepare --update-lock` 显式重新解析并取回依赖；普通准备不会偷偷更新已有锁。检查使用已安装的工具和依赖缓存，不安装工具、不下载依赖、不修改或刷新来源。Bun、WASM、wasm-bindgen、Binaryen 和 Phoenix UI 构建资源不是 diagram 的前置条件。来源故障回归可单独执行：

```sh
node --test tools/phoenix-diagram/upstream/source-check.test.mjs tools/phoenix-diagram/upstream/prepare.test.mjs
```

跨机器取回时，从本仓相同提交取回整个 upstream 目录。3.1.0 没有可核实的上游 commit，不能以远端 latest 替代该归档；缺失或摘要改变时校验停止。legacy 的每一条 manifest 路径对应固定提交下 `skills/archify/` 的同名文件。

归档可恢复到新建的临时目录：

```sh
PHOENIX_DIAGRAM_REFERENCE_DIR="$(mktemp -d)"
tar -xzf tools/phoenix-diagram/upstream/archify-3.1.0-source.tar.gz -C "$PHOENIX_DIAGRAM_REFERENCE_DIR"
```

校验器在内存重建归档文件集合，并逐项核对 manifest、成员元数据、内容摘要和许可副本。原 `local/archify-main` 存在时也须匹配；原目录缺席时仍能从冻结归档完成校验。归档、legacy 和开发测试不属于消费者 npm 资源。
