# 中间继电器-双常开@1

用于一组线圈联动两组常开触点的图面。第一组为 `NO1-A` 和 `NO1-B`，第二组为 `NO2-A` 和 `NO2-B`，线圈为 `COIL-A` 和 `COIL-B`。实物端子编号和线圈规格按所用型号核对。

## 使用方法

按所接触点的实际两端建立网络。下例的第一组触点用 `relay.NO1-A` 和 `relay.NO1-B` 引用。

```json
{"id":"relay","reference":"K12","template":"中间继电器-双常开@1","sheet":"P01"}
```

[端子定义](part.json) · [符号](symbol.svg)
