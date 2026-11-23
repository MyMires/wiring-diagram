# 气阀控制@1

用于复用参考图中的双启动、行程检测和急停组合线路。外部端子为 `MODULE_PWR_NEG`（模块电源负端）、`SOLENOID_NEG`（电磁阀负端）和 `LIMIT_DETECT`（行程检测）。内部触点关系见符号和端子定义的注记。

## 使用方法

四个参数分别设置内部开关显示位号，均接受 1–999 的整数，同一实例内位号须互异。`left_start`、`right_start`、`limit_switch`、`emergency_stop` 的默认值依次为 2、3、4、5。更改位号后仍按相同的外部端子 ID 引用。

```json
{"id":"valve","reference":"VALVE1","template":"气阀控制@1","sheet":"P01","parameters":{"left_start":12,"right_start":13,"limit_switch":14,"emergency_stop":15}}
```

[端子定义](part.json) · [线路符号](symbol.svg) · [位号配置](channel-layout.json)
