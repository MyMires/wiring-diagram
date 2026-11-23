# 电感@1

用于绘制两端式电感，逻辑端子为 `1` 和 `2`。

## 使用方法

在工程网络中引用 `inductor.1` 和 `inductor.2`。用 `note` 记录实际电感量和额定电流。

```json
{"id":"inductor","reference":"L1","template":"电感@1","sheet":"P01","note":"10 μH / 2 A"}
```

[端子定义](part.json) · [符号](symbol.svg)
