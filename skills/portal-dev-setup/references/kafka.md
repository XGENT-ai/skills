# 平台 Kafka 接入指引

平台为每个 App 供给一个 Kafka 用户及限定前缀的权限。App 只使用自己的五个环境变量；不要申请或读取 `KAFKA_ADMIN_PASSWORD`。

## 清单与环境变量

以 `listingKey: "order-worker"` 为例，清单声明：

```json
{
  "requiredServices": [{ "kind": "kafka", "name": "main" }],
  "requiredEnv": [
    "ORDER_WORKER_KAFKA_BROKERS",
    "ORDER_WORKER_KAFKA_USERNAME",
    "ORDER_WORKER_KAFKA_PASSWORD",
    "ORDER_WORKER_KAFKA_SASL_MECHANISM",
    "ORDER_WORKER_KAFKA_TOPIC_PREFIX"
  ]
}
```

`requiredServices` 的每项只写 `kind`、`name`、可选 `note`，不写地址、密码或 `envKey`。生产由平台管理员登记 Kafka 服务与自动供给档案，批准应用时选择档案、建用户及授权。也兼容只在 `requiredEnv` 声明约定键名的应用；完整列出五键便于交付检查。

| 键后缀 | 用途 / 例子 |
| --- | --- |
| `_KAFKA_BROKERS` | 逗号分隔的 `host:port`；容器一盒为 `kafka:9092` |
| `_KAFKA_USERNAME` | `app_order_worker`；连字符转换成下划线 |
| `_KAFKA_PASSWORD` | 平台供给的密码，不写进清单、源码或日志 |
| `_KAFKA_SASL_MECHANISM` | 固定 `SCRAM-SHA-512` |
| `_KAFKA_TOPIC_PREFIX` | `app.order-worker.`；保留原始 listingKey 的连字符与末尾点 |

环境变量前缀是 listingKey 大写、连字符改成下划线。**主题、消费组、事务 ID 都必须使用 `_KAFKA_TOPIC_PREFIX`**，不能从用户名反推。例如主题 `app.order-worker.events`，消费组 `app.order-worker.readers`，事务 ID `app.order-worker.tx-01`。

## 客户端配置与主题创建

协议为 `SASL_PLAINTEXT`，认证为 `SCRAM-SHA-512`，不是管理员使用的 PLAIN。客户端必须支持 SCRAM-SHA-512，并在重连时继续使用收到的 broker 广告地址。地址不含 `http://`。

App 安装 `@platformatic/kafka`（`bun add @platformatic/kafka`），将以下示例保存为 `kafka-check.ts`，在已注入五键的 App 容器中运行 `bun kafka-check.ts`。它用 **App 用户**直接向前缀内新主题发送消息，再读回一条；不预建主题：

```ts
import { Consumer, Producer, stringDeserializers, stringSerializers } from "@platformatic/kafka";

function required(key: string): string {
  const value = process.env[key];
  if (!value) throw new Error(`缺少 ${key}`);
  return value;
}
const prefix = required("ORDER_WORKER_KAFKA_TOPIC_PREFIX");
if (required("ORDER_WORKER_KAFKA_SASL_MECHANISM") !== "SCRAM-SHA-512") {
  throw new Error("Kafka 认证机制不匹配");
}
const connection = {
  clientId: "order-worker",
  bootstrapBrokers: required("ORDER_WORKER_KAFKA_BROKERS").split(",").map(s => s.trim()),
  sasl: {
    mechanism: "SCRAM-SHA-512" as const,
    username: required("ORDER_WORKER_KAFKA_USERNAME"),
    password: required("ORDER_WORKER_KAFKA_PASSWORD"),
  },
};
const topic = `${prefix}events`;
const producer = new Producer({ ...connection, autocreateTopics: true, serializers: stringSerializers });
try {
  await producer.send({ messages: [{ topic, key: "order-42", value: JSON.stringify({ eventId: "unique-event-id" }) }] });
} finally {
  await producer.close();
}
const consumer = new Consumer({
  ...connection, groupId: `${prefix}readers`, deserializers: stringDeserializers,
});
const stream = await consumer.consume({
  topics: [topic], mode: "manual", autocommit: false,
  offsets: [{ topic, partition: 0, offset: 0n }],
});
try {
  for await (const message of stream) {
    console.log(message.key, message.value);
    await message.commit();
    break;
  }
} finally {
  await stream.close();
  await consumer.close();
}
```

`@platformatic/kafka` 客户端默认不请求自动建主题，示例显式设置 `autocreateTopics: true`。平台已打开自动建主题：App 可直接向自己前缀内的新主题发送消息，自动建出的主题为 1 分区、1 副本。需要其他分区数时，仍在首次部署或迁移阶段显式创建，副本数为 1。主题名拼错也会建出新主题，不会报错；发送前核对名称。保留期由平台统一设为 168 小时，创建时不传 `retention.ms` 等保留配置，需要调整时联系平台管理员。

上面的检查从分区 0 的开头读一条消息，应该看到 `order-42` 和发送的 JSON。业务消费者通常沿用已提交的 offset，不照抄检查用的 `offset: 0n`，业务处理成功后才提交。

需要其他分区数时，在该主题收到第一条消息之前，用相同的 App 连接显式创建；例如：

```ts
import { Admin } from "@platformatic/kafka";
const admin = new Admin(connection);
try {
  await admin.createTopics({ topics: [`${prefix}partitioned-events`], partitions: 3, replicas: 1 });
} finally {
  await admin.close();
}
```

重跑创建操作只把「主题已存在」当成功，其他错误保留并处理。

| 资源（均为 App 前缀） | 授权操作 |
| --- | --- |
| 主题 | CREATE、DELETE、DESCRIBE、DESCRIBE_CONFIGS、READ、WRITE |
| 消费组 | READ、DESCRIBE、DELETE |
| 事务 ID | WRITE、DESCRIBE |

不授权 ALTER、ALTER_CONFIGS、ALL：App 不能修改主题保留期或增加分区，也不能创建或读写他人主题、加入他人消费组、使用他人事务 ID。幂等生产可用；业务仍需处理重复投递。

## 一盒联调

使用新版 `portal-dev-setup` 的公开脚本：

```bash
S=/path/to/portal-dev-setup/scripts/onebox.sh
"$S" add order-worker --manifest ./app.manifest.json --image <已交付的App镜像>
```

`add` 在清单包含 `<PREFIX>_KAFKA_USERNAME` 或 `requiredServices.kind=kafka` 时启动 Kafka，等待监听，创建 SCRAM-SHA-512 用户（8192 次迭代）和三组前缀 ACL，再把五键写入 `compose.env`。随机密码在第一次写 broker 之前保存；重复 `add` 和部分失败后的重试都沿用它，不会更换已连接客户端的密码。写密钥只显示 `***`。不要手工删掉已保存的密码行后重跑。

容器内连接 `kafka:9092`。宿主热重载进程使用同一用户和密码，但将 BROKERS 改为 `127.0.0.1:<compose.env 的 KAFKA_PORT>`；不要把这一宿主地址写回容器用的五键。`doctor` 只检查 TCP 监听，绿灯不代替认证与收发验证。

一盒将整份 `compose.env` 注入容器，**不提供 App 容器之间的密钥隔离**；仅用于可信团队的本地联调。App 自律只读自己的五键，权限验证也只用该用户，不能把管理员凭据交给 App 客户端。

### 一盒故障恢复

供给失败只告警，门户与应用初始化继续；依赖 Kafka 的 App 可能暂时不可用。先看 `"$S" dc logs --tail 50 kafka`，检查资产版本、磁盘与管理密码是否匹配，再粘贴告警中的 `add` 命令重试。旧资产先 `"$S" upgrade --image <新版一盒镜像>`；保留原 `compose.env` 与 Kafka 卷，不能改 `KAFKA_CLUSTER_ID`。

需要手工确认或修复时，下面的命令只从本地配置读取密码，命令行不含密码字面量。把 `BOX`、`KEY` 改为自己的目录和 listingKey；脚本只接受自动生成的十六进制 / base64 / URL-safe App 密码，不自动覆盖不受支持的自定义密码。

```bash
BOX=./portal-onebox
KEY=order-worker
P=$(printf '%s' "$KEY" | tr 'a-z-' 'A-Z_')
PASSWORD=$(awk -v key="${P}_KAFKA_PASSWORD" 'index($0,key"=")==1 {v=substr($0,length(key)+2)} END {print v}' "$BOX/compose.env")
test -n "$PASSWORD" || { echo '先重跑 add，保留它生成的密码'; exit 1; }
printf '%s\n' "$PASSWORD" | XGENT_ONEBOX_HOME="$BOX" "$S" dc exec -T kafka bash -ec '
  umask 077; IFS= read -r password
  dir=$(mktemp -d /tmp/xgent-kafka-admin.XXXXXX)
  trap '\''rm -rf "$dir"'\'' EXIT
  trap '\''exit 1'\'' HUP INT TERM
  jaas=$KAFKA_LISTENER_NAME_INTERNAL_PLAIN_SASL_JAAS_CONFIG
  printf "security.protocol=SASL_PLAINTEXT\nsasl.mechanism=PLAIN\nsasl.jaas.config=%s\n" "${jaas//\\/\\\\}" > "$dir/admin.properties"
  printf "SCRAM-SHA-512=iterations=8192,password=%s\n" "$password" > "$dir/user.properties"
  /opt/kafka/bin/kafka-configs.sh --bootstrap-server kafka:9092 --command-config "$dir/admin.properties" --alter --add-config-file "$dir/user.properties" --entity-type users --entity-name "app_${1//-/_}"
  acl=(--bootstrap-server kafka:9092 --command-config "$dir/admin.properties" --add --allow-principal "User:app_${1//-/_}" --allow-host "*" --resource-pattern-type prefixed)
  /opt/kafka/bin/kafka-acls.sh "${acl[@]}" --topic "app.$1." --operation Create --operation Delete --operation Describe --operation DescribeConfigs --operation Read --operation Write
  /opt/kafka/bin/kafka-acls.sh "${acl[@]}" --group "app.$1." --operation Read --operation Describe --operation Delete
  /opt/kafka/bin/kafka-acls.sh "${acl[@]}" --transactional-id "app.$1." --operation Write --operation Describe
' bash "$KEY"
unset PASSWORD
# 用原 manifest / image 参数重跑，让 add 补齐五键并重建 App 容器。
XGENT_ONEBOX_HOME="$BOX" "$S" add "$KEY" --manifest ./app.manifest.json
```

## 可靠性与网络边界

当前平台 Kafka 是单节点，重启会短暂不可用；没有高可用承诺。按 at-least-once 设计，消费方用业务事件 ID 去重、成功处理后提交 offset，并实现重连与重试。Kafka 不是业务记录系统，权威数据仍在业务数据库 / 对象存储；消息过期后的补偿由 App 自己实现。

隔离粒度是 **App，不是租户**。一个 App 内不同租户的消息，租户字段、消费授权及数据隔离由 App 负责。

SASL_PLAINTEXT 只认证，不加密消息。HOST 和一盒 EXTERNAL 默认回环；跨 VM 接入由运维同时配置外部监听、客户端可达的广告地址与限来源安全组，不能只改 bootstrap 地址。不要自行开放公网端口。备份 / 恢复必须同时保留 Kafka 卷、`KAFKA_CLUSTER_ID` 和配置，不能把同一个卷搭配新集群 ID。
