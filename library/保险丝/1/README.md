# 保险丝@1

用于绘制参考线路中的串联熔断器。两端为 `1` 和 `2`。

## 使用方法

保险丝两侧按参考图建立各自的网络，分别引用 `fuse.1` 和 `fuse.2`。`note` 可记录已确认的额定电流、电压和熔断特性。

```json
{"id":"fuse","reference":"F1","template":"保险丝@1","sheet":"P01","note":"1 A"}
```

[端子定义](part.json) · [符号](symbol.svg)
