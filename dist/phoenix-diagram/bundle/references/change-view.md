# 变更视图（workflow、lifecycle、tree、erd）

要讲「改完以后和现在差在哪」时，在 workflow、lifecycle、tree、erd 的一份输入里标记新增、删除、变更；引擎画出变更视图，并可切换到变更前、变更后。输入见各图型 schema 与最小示例；标记的取值由 [common schema](../schemas/common.schema.json) 的 `elementChange` 与 `relationChange` 约定。

- 适用对象：workflow 的 `nodes` 与 `edges`、lifecycle 的 `states` 与 `transitions`、erd 的 `entities`、`entities[].attributes` 与 `relationships`、tree 的 `nodes`。节点可写 `change: "added" | "removed" | "changed"`；关系只写 `added` 或 `removed`——一条关系的「变更」写成删除旧的、新增新的。
- 标记只取自授权材料；不推断改动的原因、风险或影响。「变更」只表示该项有变化：三种视图显示的都是输入里的同一份描述，不接受旧值字段。
- 端点规则：被标为删除或新增的节点，与它相连的关系要写成对应的新增或删除，否则校验失败（诊断码 `change/dangling-relation`）。
- erd 字段继承：实体带标记时全部字段继承同一标记；字段写了与实体不同的标记会被拒绝（`change/conflict`）。
- tree 换父：保留节点写 `parent_before` 指变更前的父节点，`parent` 写新父节点；两个父节点必须不同。变更前视图同样无环（既有码 `tree/cycle`）；树根不能写 `parent_before`。容器不能标记。父节点被标为新增或删除时，子节点要随它标为同一标记，或写 `parent_before` 换到仍存在的父节点；否则连线会在另一个视图里指向不存在的节点（`change/dangling-relation`）。
- `mainPath` 与 `semanticChecks` 按全部元素书写，不按视图拆分。
- 读者看到什么：默认的变更视图给新增、删除、变更的项加徽标并在页首显示计数，图下方是三组视图说明；变更前、变更后两个视图里不存在于该视图的元素处留空；导出 SVG/PNG 与打印是当前所选视图；静态图是变更视图。
- 交付答复说明三类变更各有多少、有没有标为变更的项。

品牌与本地预览仍按各自页面执行；两份输入的比较走[架构比较](architecture-compare.md)，不与变更标记混用。
