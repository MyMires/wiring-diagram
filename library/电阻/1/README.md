# 电阻@1

用于绘制普通两端电阻。`1` 和 `2` 是图面的逻辑端子。

## 使用方法

在工程网络中引用 `resistor.1` 和 `resistor.2`。用 `note` 记录实际阻值和额定功率。

```json
{"id":"resistor","reference":"R1","template":"电阻@1","sheet":"P01","note":"1 kΩ / 0.25 W"}
```

[端子定义](part.json) · [符号](symbol.svg)
