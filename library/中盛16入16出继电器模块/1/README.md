# 中盛16入16出继电器模块@1

用于绘制该型号的数字输入、继电器输出和通信接线。`Xn` 是数字输入，`COM` 是数字输入公共端。每组输出有 `COMn`、`NOn` 和 `NCn`。固定接口包括 `A/H`、`B/L`、`DC+` 和 `DC-`。

## 使用方法

`inputs` 和 `outputs` 均支持 1–16，默认各 16。减少通道数会移除超出配置的编号端子。`station` 用文本填写以保留前导零。新工程使用 `DC+` 和 `DC-`，旧工程的 `V+` 和 `V-` 仍可解析。

```json
{"id":"relay","reference":"K1","template":"中盛16入16出继电器模块@1","sheet":"P01","station":"01","parameters":{"inputs":8,"outputs":8}}
```

查询配置后可用的端子。

```sh
python wiring.py part 中盛16入16出继电器模块@1 --parameters '{"inputs":8,"outputs":8}'
```

[端子定义](part.json) · [默认符号](symbol.svg) · [通道配置](channel-layout.json)
