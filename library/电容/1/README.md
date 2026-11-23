# 电容@1

用于绘制无极性电容，两端为 `1` 和 `2`。电容量和额定电压可在图面备注中显示。

## 使用方法

在工程网络中引用 `capacitor.1` 和 `capacitor.2`。把实际规格写入 `note`。

```json
{"id":"capacitor","reference":"C1","template":"电容@1","sheet":"P01","note":"100 nF / 50 V"}
```

[端子定义](part.json) · [符号](symbol.svg)
