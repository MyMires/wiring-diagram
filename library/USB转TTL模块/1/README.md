# USB转TTL模块@1

用于绘制 USB 转 TTL 串口转接。`USB` 作为整口引用，TTL 侧有 `TXD`、`RXD`、`GND`、`3V` 和 `5V`，对应具体接线以参考资料为准。

## 使用方法

将实例放入工程的 `instances`。网络可引用 `adapter.TXD`、`adapter.RXD` 或 `adapter.USB` 等端点。

```json
{"id":"adapter","reference":"USB1","template":"USB转TTL模块@1","sheet":"P01"}
```

[端子定义](part.json) · [符号](symbol.svg)
