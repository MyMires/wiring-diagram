# 测试治具接线图初版

使用 Python 3.11+，在独立接线图工作区运行。PDF 引擎为 [`pymupdf-lite`](https://pypi.org/project/pymupdf-lite/2.1/)。

`library/` 存可复用的模块模板，`projects/` 存具体的接线工程和生成图。两者的子目录都可以用中文。模块目录名与 `part.json` 的 `id` 一致后，工程可用 `"template": "气阀控制"` 引用该型号的最高数字版本，或用 `"template": "气阀控制@1"` 固定为第 1 版。模块显示名称可通过实例的 `name` 单独设置。

```sh
python -m pip install -r requirements.txt
python wiring.py part
python wiring.py part 1bxrr16
python wiring.py part 1bxrr16@1
python wiring.py validate projects/lamp/project.json
python wiring.py render projects/lamp/project.json projects/lamp/wiring.svg
python wiring.py render projects/lamp/project.json projects/lamp/wiring.pdf
```

[`projects/lamp`](projects/lamp/) 包含灯板通信接线摘录和可重新出图的工程。`part` 命令返回可编辑文本字段、端子定义和支持的图面。实例通过 `view` 选择 `symbol` 或 `photo`，两种图面共用端子 ID。SVG 符号和实物照片的相对坐标均在模板内部，网络引线起于所选图面的端子锚点。样例中的 `X1` 使用实物模板，照片上标记已接线的 `A`、`B`。

在 `sheets` 声明图页，并给同页配件填写相同的 `instances[].sheet`。生成器根据模块、标题和网名的尺寸紧凑排版。比如 [`project-one-page.json`](projects/lamp/project-one-page.json) 将继电器板和转接器放在同一页，自动生成 [`wiring-one-page.svg`](projects/lamp/wiring-one-page.svg)。

[`projects/zhongsheng-voltage`](projects/zhongsheng-voltage/) 是 16 入 16 出继电器板和 8 通道电压采集模块的单页接线。网络可使用中文原名。若参考图只给出本工程内的一个端子及外部网名，在该网络设置 `"external": true`，并保留本工程已知的端子。

中盛模板的实例可以填写 `"station": "01"`。继电器板可在 `parameters` 中指定 `inputs`、`outputs`（各 1–16），电压采集模块可指定 `channels`（1–8）。省略参数时使用原图的完整通道数。减少通道会裁掉编号超出配置的端子并重新排版，保留其余端子的 ID、通信和供电接口。若网络仍引用已裁掉的端子，校验会报错。当前范围来自参考模板，其他实体型号的管脚以其资料为准。

用 `python wiring.py part zhongsheng-voltage-8ch@1 --parameters '{"channels":4}'` 查询指定通道数的可用端子。参见 [参数样例](projects/zhongsheng-voltage/project-channels.json)及其[出图](projects/zhongsheng-voltage/wiring-channels.svg)。

`limit-switch@1` 可由 `parameters.count` 生成 1–9 只行程开关，由 `parameters.start` 指定起始编号，默认从 S1 开始。每只开关提供独立的 1、2、3 端子，例如 `switches.S2-2`。在工程 `nets` 中填写所需接线，参见 [S2、S3 串接样例](projects/limit-switch/project.json)。

[`switching-power-supply@1`](library/switching-power-supply/1/part.json) 提供 `L`、`N`、`PE`、`+V`、`-V`。可用实例的 `name` 和 `note` 显示设备名称和输出规格。[气阀控制工程](projects/air-valve-control/project.json) 展示 `air-valve-control@1` 的三个外部端子和四个可配置的内部开关位号。

`render` 根据扩展名出图。多页 SVG 使用 `wiring-p01.svg`、`wiring-p02.svg` 等文件名。SVG 文字以矢量轮廓输出，以保持显示一致，原文保存在图的描述元素中。

[测试接线 Skill](skills/test-wiring/SKILL.md) 指导 AI 为测试台架、设备、模块和治具建库、录入接线并出图。

首版使用 A4 横向、按工程指定图页排版。配件实物模板随实例放置，图页还可通过 `photo` 附一张说明照片。达到可读尺寸下限仍放不下时，校验会指出需要拆分的图页。同一工程中，标有同一网络名的端子属于同一个网络，适用于同页和跨页接线。`validate` 输出每个网络的完整成员。

## 工程配色

在工程 JSON 顶层填写 `colors`。`internal` 控制模块内导线、端点、端子文字和照片接点标记，`external` 控制外部引线和网名。两项都接受 `#RRGGBB`，也都可以单独省略。缺省项使用蓝色 `#1C5C94`，省略整个 `colors` 时两类线路均用该蓝色。接头和普通端子使用同一内部颜色。

[四模块工程](projects/four-modules/project.json) 只指定 `"colors": {"external": "#9A4A16"}`，因此模块内部用默认蓝色，外部用自定义深橙色。气阀控制、UUT 电源、USB 转 TTL 和行程开关实例都指向 `P01`，由工具自动排在同一张 A4 上。[查看生成的 SVG](projects/four-modules/wiring.svg)

```sh
python wiring.py validate projects/four-modules/project.json
python wiring.py render projects/four-modules/project.json projects/four-modules/wiring.svg
```
