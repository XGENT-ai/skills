# 离线演练：填写并预览

```formal
REQUIREMENT @preview_result:
  ACCEPTANCE:
    GIVEN: input_filled_with_city
    WHEN: preview_clicked
    EXPECT: status_contains_entered_city
```

下面的 POSIX shell 示例可离线运行。页面只有一个城市输入框和一个预览按钮，适合验收新安装。给临时会话取独立名称，后续命令都沿用它。

```sh
ab_session="ab-demo-$$"
agent-browser --session "$ab_session" open 'data:text/html,<label>City <input id="city"></label><button onclick="this.nextElementSibling.textContent=city.value">Preview</button><output></output>'
agent-browser --session "$ab_session" snapshot -i
```

从输出找到 `textbox "City"` 和 `button "Preview"`。例如它们显示 `[ref=e2]`、`[ref=e1]` 时，对应命令参数为 `@e2`、`@e1`。**按本次快照替换下面的两个编号**，再执行操作。

```sh
agent-browser --session "$ab_session" fill @e2 "Shanghai"
agent-browser --session "$ab_session" click @e1
agent-browser --session "$ab_session" snapshot
```

结果快照中，`status` 节点下应出现 `Shanghai`。输入框里有相同的值仅说明填写成功。若需要截图，在此时运行 `agent-browser --session "$ab_session" screenshot preview.png`。演示结束后运行 `agent-browser --session "$ab_session" close`。
