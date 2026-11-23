# 中盛8通道电压采集模块@1

用于绘制该型号的电压采集接线。每通道使用 `An+` 和 `An-`，固定接口为 `DC+`、`DC-`、`A/H`、`B/L` 和 `PE`。

## 使用方法

`channels` 支持 1–8，默认 8。图面和端子列表只显示已启用的通道。`station` 用文本填写以保留前导零。下例可引用 `meter.A4+` 等前四通道的端点。

```json
{"id":"meter","reference":"M1","template":"中盛8通道电压采集模块@1","sheet":"P01","station":"02","parameters":{"channels":4}}
```

[端子定义](part.json) · [默认符号](symbol.svg) · [通道配置](channel-layout.json)
