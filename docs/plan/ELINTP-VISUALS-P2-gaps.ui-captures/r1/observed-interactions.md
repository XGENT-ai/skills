# R1 · 实际操作记录

本文件只记录实际执行的动作与观察结果；未执行的动作不作推断。

## 1. U1

- 动作：旧 P2 页面实际标注普通段落并主动提交
- 观察：

```json
{
  "blockId": "s1:b1",
  "quote": "这是旧版已经能够登记的普通段落，用来核对升级前后的块身份与标注位置。",
  "note": "旧标注仍应附着原段落",
  "sentStatus": "已提交给 Agent"
}
```

## 2. U1

- 动作：当前版以 --previous 打开旧结果并开启标注
- 观察：

```json
{
  "oldBlockId": "s1:b1",
  "summary": "旧标注仍应附着原段落",
  "emptyCaptionControls": 2,
  "round": 2
}
```

## 3. U1

- 动作：同稿、同输出路径、同端口从旧 P2 重开当前版并恢复原浏览器草稿
- 观察：

```json
{
  "samePageKey": true,
  "samePort": true,
  "oldBlockId": "s1:b1",
  "restoredNote": "旧标注仍应附着原段落"
}
```

## 4. U1

- 动作：1440/light 打开页面并用三次 Tab、Enter 开启标注
- 观察：

```json
{
  "initial": {
    "togglePressed": "false",
    "submitDisabled": true,
    "status": "先选择内容块并添加标注",
    "dimensions": {
      "viewport": 1440,
      "rootScrollWidth": 1440,
      "bodyScrollWidth": 1200,
      "background": "rgb(250, 250, 247)",
      "figures": [
        {
          "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
          "caption": "",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": null,
          "caption": "暗色流程截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
          "caption": "",
          "imageLoaded": null,
          "scrollWidth": null,
          "clientWidth": null
        },
        {
          "ref": null,
          "caption": "大尺寸验证截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": null,
          "caption": "另一张暗色验证截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        }
      ]
    }
  },
  "focusSequence": [
    "跳到正文",
    "切换亮暗",
    "＋ 标注内容"
  ],
  "keyboardEnabled": "true",
  "aria": [
    "＋ 添加标注: docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
    "＋ 添加标注: docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png"
  ]
}
```

## 5. U1

- 动作：1440/dark 打开页面并用三次 Tab、Enter 开启标注
- 观察：

```json
{
  "initial": {
    "togglePressed": "false",
    "submitDisabled": true,
    "status": "先选择内容块并添加标注",
    "dimensions": {
      "viewport": 1440,
      "rootScrollWidth": 1440,
      "bodyScrollWidth": 1200,
      "background": "rgb(21, 33, 36)",
      "figures": [
        {
          "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
          "caption": "",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": null,
          "caption": "暗色流程截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
          "caption": "",
          "imageLoaded": null,
          "scrollWidth": null,
          "clientWidth": null
        },
        {
          "ref": null,
          "caption": "大尺寸验证截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        },
        {
          "ref": null,
          "caption": "另一张暗色验证截图",
          "imageLoaded": true,
          "scrollWidth": 840,
          "clientWidth": 840
        }
      ]
    }
  },
  "focusSequence": [
    "跳到正文",
    "切换亮暗",
    "＋ 标注内容"
  ],
  "keyboardEnabled": "true",
  "aria": [
    "＋ 添加标注: docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
    "＋ 添加标注: docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png"
  ]
}
```

## 6. U1

- 动作：360/light 打开页面并用三次 Tab、Enter 开启标注
- 观察：

```json
{
  "initial": {
    "togglePressed": "false",
    "submitDisabled": true,
    "status": "先选择内容块并添加标注",
    "dimensions": {
      "viewport": 360,
      "rootScrollWidth": 360,
      "bodyScrollWidth": 360,
      "background": "rgb(250, 250, 247)",
      "figures": [
        {
          "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
          "caption": "",
          "imageLoaded": true,
          "scrollWidth": 1440,
          "clientWidth": 320
        },
        {
          "ref": null,
          "caption": "暗色流程截图",
          "imageLoaded": true,
          "scrollWidth": 1440,
          "clientWidth": 320
        },
        {
          "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
          "caption": "",
          "imageLoaded": null,
          "scrollWidth": null,
          "clientWidth": null
        },
        {
          "ref": null,
          "caption": "大尺寸验证截图",
          "imageLoaded": true,
          "scrollWidth": 2048,
          "clientWidth": 320
        },
        {
          "ref": null,
          "caption": "另一张暗色验证截图",
          "imageLoaded": true,
          "scrollWidth": 2048,
          "clientWidth": 320
        }
      ]
    }
  },
  "focusSequence": [
    "跳到正文",
    "切换亮暗",
    "＋ 标注内容"
  ],
  "keyboardEnabled": "true",
  "aria": [
    "＋ 添加标注: docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
    "＋ 添加标注: docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png"
  ]
}
```

## 7. U1

- 动作：窄屏暗色用键盘打开缺失图编辑器，再按 Escape
- 观察：

```json
{
  "quote": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
  "quoteLength": 110,
  "containsDoubleSpace": true,
  "focusOutline": {
    "tag": "INPUT",
    "outline": "solid"
  },
  "focusReturned": true
}
```

## 8. U1

- 动作：360/dark 打开页面并用三次 Tab、Enter 开启标注
- 观察：

```json
{
  "initial": {
    "togglePressed": "false",
    "submitDisabled": true,
    "status": "先选择内容块并添加标注",
    "dimensions": {
      "viewport": 360,
      "rootScrollWidth": 360,
      "bodyScrollWidth": 360,
      "background": "rgb(21, 33, 36)",
      "figures": [
        {
          "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
          "caption": "",
          "imageLoaded": true,
          "scrollWidth": 1440,
          "clientWidth": 320
        },
        {
          "ref": null,
          "caption": "暗色流程截图",
          "imageLoaded": true,
          "scrollWidth": 1440,
          "clientWidth": 320
        },
        {
          "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
          "caption": "",
          "imageLoaded": null,
          "scrollWidth": null,
          "clientWidth": null
        },
        {
          "ref": null,
          "caption": "大尺寸验证截图",
          "imageLoaded": true,
          "scrollWidth": 2048,
          "clientWidth": 320
        },
        {
          "ref": null,
          "caption": "另一张暗色验证截图",
          "imageLoaded": true,
          "scrollWidth": 2048,
          "clientWidth": 320
        }
      ]
    }
  },
  "focusSequence": [
    "跳到正文",
    "切换亮暗",
    "＋ 标注内容"
  ],
  "keyboardEnabled": "true",
  "aria": [
    "＋ 添加标注: docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
    "＋ 添加标注: docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png"
  ]
}
```

## 9. U1

- 动作：禁用 JavaScript 打开当前审阅 URL
- 观察：

```json
{
  "toolbarCount": 0,
  "bodyTextLength": 404,
  "dimensions": {
    "viewport": 1440,
    "rootScrollWidth": 1440,
    "bodyScrollWidth": 1200,
    "background": "rgb(250, 250, 247)",
    "figures": [
      {
        "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
        "caption": "",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": null,
        "caption": "暗色流程截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
        "caption": "",
        "imageLoaded": null,
        "scrollWidth": null,
        "clientWidth": null
      },
      {
        "ref": null,
        "caption": "大尺寸验证截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": null,
        "caption": "另一张暗色验证截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      }
    ]
  }
}
```

## 10. U1

- 动作：在标注控件可见时生成 Chrome 打印 PDF
- 观察：

```json
{
  "pdf": "/Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P2-gaps.ui-captures/r1/logs/review-print.pdf",
  "toolbarScreenVisible": true
}
```

## 11. U1

- 动作：新增两张空图注图片标注并刷新
- 观察：

```json
{
  "restoredNotes": [
    "旧标注仍应附着原段落",
    "请重讲这张无图注图片",
    "请补图，保留这个完整原引用"
  ],
  "status": "3 条标注"
}
```

## 12. U1

- 动作：主动提交当前轮三条标注
- 观察：

```json
{
  "sentStatus": "已提交给 Agent",
  "result": [
    {
      "blockId": "s1:b1",
      "sectionId": "s1",
      "quote": "这是旧版已经能够登记的普通段落，用来核对升级前后的块身份与标注位置。",
      "kind": "question",
      "note": "旧标注仍应附着原段落"
    },
    {
      "blockId": "s1:b4",
      "sectionId": "s1",
      "quote": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
      "kind": "unclear",
      "note": "请重讲这张无图注图片"
    },
    {
      "blockId": "s2:b6",
      "sectionId": "s2",
      "quote": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
      "kind": "suggestion",
      "note": "请补图，保留这个完整原引用"
    }
  ],
  "allControlsDisabled": 3
}
```

## 13. U2

- 动作：1440/light 离线切换主题、刷新并用键盘跳到最后一个大纲项
- 观察：

```json
{
  "offline": true,
  "toggled": "dark",
  "persisted": "dark",
  "hash": "#s3",
  "current": "#s3"
}
```

## 14. U2

- 动作：1440/dark 离线切换主题、刷新并用键盘跳到最后一个大纲项
- 观察：

```json
{
  "offline": true,
  "toggled": "light",
  "persisted": "light",
  "hash": "#s3",
  "current": "#s3"
}
```

## 15. U2

- 动作：360/light 离线切换主题、刷新并查看缺失原引用
- 观察：

```json
{
  "offline": true,
  "toggled": "dark",
  "persisted": "dark",
  "missingText": "图片缺失：docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
  "dimensions": {
    "viewport": 360,
    "rootScrollWidth": 360,
    "bodyScrollWidth": 360,
    "background": "rgb(21, 33, 36)",
    "figures": [
      {
        "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
        "caption": "",
        "imageLoaded": true,
        "scrollWidth": 1440,
        "clientWidth": 320
      },
      {
        "ref": null,
        "caption": "暗色流程截图",
        "imageLoaded": true,
        "scrollWidth": 1440,
        "clientWidth": 320
      },
      {
        "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
        "caption": "",
        "imageLoaded": null,
        "scrollWidth": null,
        "clientWidth": null
      },
      {
        "ref": null,
        "caption": "大尺寸验证截图",
        "imageLoaded": true,
        "scrollWidth": 2048,
        "clientWidth": 320
      },
      {
        "ref": null,
        "caption": "另一张暗色验证截图",
        "imageLoaded": true,
        "scrollWidth": 2048,
        "clientWidth": 320
      }
    ]
  }
}
```

## 16. U2

- 动作：360/dark 离线切换主题、刷新并查看缺失原引用
- 观察：

```json
{
  "offline": true,
  "toggled": "light",
  "persisted": "light",
  "missingText": "图片缺失：docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
  "dimensions": {
    "viewport": 360,
    "rootScrollWidth": 360,
    "bodyScrollWidth": 360,
    "background": "rgb(250, 250, 247)",
    "figures": [
      {
        "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
        "caption": "",
        "imageLoaded": true,
        "scrollWidth": 1440,
        "clientWidth": 320
      },
      {
        "ref": null,
        "caption": "暗色流程截图",
        "imageLoaded": true,
        "scrollWidth": 1440,
        "clientWidth": 320
      },
      {
        "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
        "caption": "",
        "imageLoaded": null,
        "scrollWidth": null,
        "clientWidth": null
      },
      {
        "ref": null,
        "caption": "大尺寸验证截图",
        "imageLoaded": true,
        "scrollWidth": 2048,
        "clientWidth": 320
      },
      {
        "ref": null,
        "caption": "另一张暗色验证截图",
        "imageLoaded": true,
        "scrollWidth": 2048,
        "clientWidth": 320
      }
    ]
  }
}
```

## 17. U2

- 动作：离线且禁用 JavaScript 打开终稿
- 观察：

```json
{
  "offline": true,
  "bodyTextLength": 404,
  "dimensions": {
    "viewport": 1440,
    "rootScrollWidth": 1440,
    "bodyScrollWidth": 1200,
    "background": "rgb(250, 250, 247)",
    "figures": [
      {
        "ref": "docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png",
        "caption": "",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": null,
        "caption": "暗色流程截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": "docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
        "caption": "",
        "imageLoaded": null,
        "scrollWidth": null,
        "clientWidth": null
      },
      {
        "ref": null,
        "caption": "大尺寸验证截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      },
      {
        "ref": null,
        "caption": "另一张暗色验证截图",
        "imageLoaded": true,
        "scrollWidth": 840,
        "clientWidth": 840
      }
    ]
  },
  "themeButtonsVisible": 0
}
```

## 18. U2

- 动作：离线暗色上下文生成 Chrome 打印 PDF
- 观察：

```json
{
  "offline": true,
  "pdf": "/Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P2-gaps.ui-captures/r1/logs/final-print.pdf"
}
```

## 19. U2

- 动作：fresh offline dark context; keyboard Enter final outline
- 观察：

```json
{
  "action": "fresh offline dark context; keyboard Enter final outline",
  "theme": null,
  "systemDark": true,
  "hash": "#s3",
  "background": "rgb(21, 33, 36)"
}
```

## 20. U2

- 动作：fresh offline dark context; scroll to missing reference
- 观察：

```json
{
  "action": "fresh offline dark context; scroll to missing reference",
  "theme": null,
  "systemDark": true,
  "missingText": "图片缺失：docs/diagrams/missing/empty-caption  diagram  with-a-very-long-name-that-exceeds-eighty-unicode-characters.png",
  "background": "rgb(21, 33, 36)",
  "rootScrollWidth": 360,
  "viewport": 360
}
```

## 21. U1

- 动作：编辑、删除、清除全部后重新添加一条标注；停止本机服务并主动提交
- 观察：

```json
{
  "edited": "服务不可达时保留这条标注",
  "afterDelete": {
    "status": "先选择内容块并添加标注",
    "submitDisabled": true
  },
  "afterClear": {
    "status": "先选择内容块并添加标注",
    "submitDisabled": true,
    "stored": "{\"items\":[]}"
  },
  "failure": {
    "toolbarText": "＋ 标注内容\n↑ 提交给 Agent\n× 清除全部\n1 条标注\n\n本机服务不可达，标注已保留。请让 Agent 用原端口重开本轮后再提交。",
    "alerts": [
      "本机服务不可达，标注已保留。请让 Agent 用原端口重开本轮后再提交。"
    ],
    "status": "1 条标注",
    "summary": "服务不可达时保留这条标注",
    "submitDisabled": false,
    "clearDisabled": false,
    "annotateDisabled": false
  },
  "resultExists": false
}
```
