# 治具供电@1

用于绘制参考图里的 AC 220 V 电源插座和双极电源开关组合线路。`L` 和 `N` 引出自联动触点的开关侧，`PE` 直通。图中保留 1 A 保险丝的原图注记，原图未标示它的接入位置。

## 使用方法

`main_switch` 设定内部电源开关的显示编号，支持 1–999，默认显示 `S1`。下例显示 `S7`，工程仍用 `power.L`、`power.PE`、`power.N` 录入电源网络。

```json
{"id":"power","reference":"PWR1","template":"治具供电@1","sheet":"P01","parameters":{"main_switch":7}}
```

[端子定义](part.json) · [线路符号](symbol.svg) · [位号配置](channel-layout.json)
