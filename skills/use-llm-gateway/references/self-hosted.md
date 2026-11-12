# 自托管决策与搜索接入

只有正在配置自托管供应商时读本页。网关不代管实例，部署、改配置和重启须在当前任务的授权环境执行；调用方接入请求不自动授权部署上游或更改网关部署开关。

## 从网关所在位置选地址

System One与SearXNG的渠道地址必须从**网关服务端**可达。网关直接运行在同一宿主时可用localhost；网关在容器里时localhost是该容器，不能填浏览器或另一台机器的localhost。相同Docker网络内使用服务DNS名与容器端口，例如`http://searxng:8080`；通过宿主发布端口时用网关可达的宿主地址。

私网上游需要运维在网关部署设置`LLM_GATEWAY_ALLOW_PRIVATE_UPSTREAMS=true`并重启；这不是租户开关，不放宽网页读取目标的内网限制。连接测试「无法判断」只说明没有可用的免费凭证校验端点，仍可保存并通过真实调用验能力。

## 已有 System One 实例

在供应商目录选择自托管System One，填写其API基础地址（最终请求为该地址加`/systemone`），Key按实例要求填写，允许留空。没有目录模型时，在接入完成页选择配置路由、创建决策模型，手填实例接受的上游模型名；录Token采购价。若租户接入平台金额预算而实例无计量合同，需明确录全0采购价，不能拿参考实现上下文长度当任意兼容实例的硬限保证。

## 准备本机 SearXNG

在已有Docker的机器上，为验证实例准备配置与数据目录；以下使用官方GHCR镜像：

```sh
mkdir -p searxng/config searxng/data
docker run -d --name llmgw-searxng \
  -p 8888:8080 \
  -v "$PWD/searxng/config:/etc/searxng" \
  -v "$PWD/searxng/data:/var/cache/searxng" \
  -e SEARXNG_SECRET="$(openssl rand -hex 32)" \
  ghcr.io/searxng/searxng:latest
```

实例默认只开HTML。编辑镜像创建的`searxng/config/settings.yml`，在**现有**search节里将formats改为以下列表，不另建重复search节：

```yaml
search:
  formats:
    - html
    - json
```

保存后重启，在网关所在机器或容器内请求JSON：

```sh
docker restart llmgw-searxng
# 网关直接运行在同一宿主时使用此地址；容器网关换成其可达地址。
curl -i --get 'http://localhost:8888/search' \
  --data-urlencode 'q=SearXNG' --data-urlencode 'format=json'
```

预期200及JSON的results数组；403先核对是否启用了json并重启。空数组是搜索完成，不能证明外部引擎正常，查看容器日志与实例出站网络。

租户管理员从供应商目录接入SearXNG，地址填实例根地址、不带/search，Key留空。若同Docker网络下容器名为llmgw-searxng，填`http://llmgw-searxng:8080`；与宿主直接运行的网关连接则填`http://localhost:8888`。连接测试「无法判断」不阻断保存。同步后给web-search建搜索路由、设默认搜索路由，为供应商web-search录按次采购价（免费实例明确录0）。

用搜索调试台输入真实查询，或按[api.md](api.md#搜索与读取)调用/v1/tools/search；检查实际provider、结果与日志。SearXNG不支持读取，全文需另接支持fetch的供应商并设置默认读取路由。

镜像与挂载方式依据[SearXNG容器安装](https://docs.searxng.org/admin/installation-docker.html)，JSON设置依据[search配置](https://docs.searxng.org/admin/settings/settings_search.html)。
