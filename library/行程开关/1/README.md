# 行程开关@1

用于绘制一只或多只同型行程开关。每只有独立的 1、2、3 端子，其中 2 为公共端。触点状态和实际串并接关系按参考图录入。

## 使用方法

`count` 支持 1–9，默认 1。`start` 支持 1–999，默认 1。下例生成 S2 和 S3，网络可分别引用 `switches.S2-2` 和 `switches.S3-2`。

```json
{"id":"switches","reference":"S2-S3","template":"行程开关@1","sheet":"P01","parameters":{"count":2,"start":2}}
```

[端子定义](part.json) · [单只开关符号](symbol.svg) · [重复配置](channel-layout.json)
