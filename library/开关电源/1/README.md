# 开关电源@1

用于绘制带交流输入和直流输出的开关电源。交流侧为 `L`、`N` 和 `PE`，直流侧为 `+V` 和 `-V`。具体输出规格按工程填写。

## 使用方法

用 `name` 标注用途，用 `note` 标注输出规格。下例可在网络中引用 `supply.+V` 或 `supply.-V`。

```json
{"id":"supply","reference":"PS1","template":"开关电源@1","sheet":"P01","name":"UUT电源","note":"15 V / 1 A"}
```

[端子定义](part.json) · [符号](symbol.svg)
