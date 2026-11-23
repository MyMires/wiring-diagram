# 中间继电器@1

用于表示一组线圈联动多组转换触点。符号显示未吸合状态。每组有公共端 `COMn`、常开端 `NOn` 和常闭端 `NCn`，线圈端子为 `COIL-A` 和 `COIL-B`。逻辑端子与实物针脚的对应关系按所用型号核对。

## 使用方法

`channels` 设为 1–9，默认 2。下例显示四组转换触点，网络可引用 `relay.COM4`、`relay.NO4` 等端点。

```json
{"id":"relay","reference":"K1","template":"中间继电器@1","sheet":"P01","parameters":{"channels":4}}
```

[端子定义](part.json) · [默认符号](symbol.svg) · [触点配置](channel-layout.json)
