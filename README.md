# 测试治具接线图

从器件模板和工程 JSON 生成测试治具接线图。`part` 查询模板和端子，`validate` 检查网表与排版，`render` 输出 SVG 或 PDF。发布记录与对应源码见 [GitHub Releases](https://github.com/MyMires/wiring-diagram/releases)。

## 从源码运行

在包含 `wiring.py`、`library/` 和 `projects/` 的仓库根目录运行。源码版使用 Python 3.11+ 和 `pymupdf-lite`。[PDF 引擎](https://pypi.org/project/pymupdf-lite/2.1/)

```sh
python -m pip install -r requirements.txt
python wiring.py --version
python wiring.py part
python wiring.py part 继电器板-1BXRR16
python wiring.py part 继电器板-1BXRR16@1
python wiring.py validate projects/lamp/project.json
python wiring.py render projects/lamp/project.json projects/lamp/wiring.svg
python wiring.py render projects/lamp/project.json projects/lamp/wiring.pdf
```

## Windows 单文件版

`wiring.exe` 可在 Windows x64 上单独运行，内置 Python、PDF 引擎和构建时的器件库。工程 JSON 与它引用的实物照片随工程目录保存。修改器件库时使用源码版，完成后可重新构建 EXE。

从仓库根目录使用 PowerShell 运行以下命令。

```powershell
.\dist\wiring.exe --version
.\dist\wiring.exe part
.\dist\wiring.exe validate .\projects\lamp\project.json
.\dist\wiring.exe render .\projects\lamp\project.json .\wiring.svg
```

构建机需有 Windows x64、Python 3.12 x64 和 Microsoft Visual C++ x64 运行库。从仓库根目录运行以下命令，产物为 `dist/wiring.exe`。[微软运行库下载](https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist)

```powershell
py -3.12 -m venv .venv\windows-build
.\.venv\windows-build\Scripts\python.exe -m pip install -r requirements.txt pyinstaller==6.19.0 pyinstaller-hooks-contrib==2026.3
.\.venv\windows-build\Scripts\python.exe -m PyInstaller --clean --noconfirm wiring.spec
```

## 工具版本

工具从 `0.1.0` 开始，版本号采用 `主版本.次版本.修订号`。源码命令、Windows 命令和 EXE 文件属性共用这一版本号。升级时更新 `wiring_version.py`，重新构建并核对 `--version`。Git 发布标签使用 `v<版本号>`，例如 `v0.1.0`。[版本定义](wiring_version.py)

工程 JSON 的 `version` 标识文件格式。`型号@版本` 选定器件模板版本。[工程格式](skills/test-wiring/references/projects.md) · [模板格式](skills/test-wiring/references/templates.md)

## 示例工程

`library/` 存可复用的器件模板，`projects/` 存接线工程和生成图。模板目录使用中文名称，保留实际型号和接口缩写。模板目录名与 `part.json` 的 `id` 一致，工程可用 `"template": "气阀控制"` 选最高数字版本，或用 `"template": "气阀控制@1"` 固定第 1 版。实例的 `name` 可单独设置显示名称。旧工程中的英文模板标识仍可解析。

[`projects/lamp`](projects/lamp/) 包含灯板通信接线摘录和可重新出图的工程。`part` 命令返回可编辑文本字段、端子定义和支持的图面。实例通过 `view` 选择 `symbol` 或 `photo`，两种图面共用端子 ID。SVG 符号和实物照片的相对坐标均在模板内部，网络引线起于所选图面的端子锚点。样例中的 `X1` 使用实物模板，照片上标记已接线的 `A`、`B`。

在 `sheets` 声明图页，并给同页配件填写相同的 `instances[].sheet`。生成器根据模块、标题和网名的尺寸紧凑排版。比如 [`project-one-page.json`](projects/lamp/project-one-page.json) 将继电器板和转接器放在同一页，自动生成 [`wiring-one-page.svg`](projects/lamp/wiring-one-page.svg)。

[`projects/zhongsheng-voltage`](projects/zhongsheng-voltage/) 是 16 入 16 出继电器板和 8 通道电压采集模块的单页接线。网络可使用中文原名。若参考图只给出本工程内的一个端子及外部网名，在该网络设置 `"external": true`，并保留本工程已知的端子。

中盛模板的实例可以填写 `"station": "01"`。继电器板可在 `parameters` 中指定 `inputs`、`outputs`（各 1–16），电压采集模块可指定 `channels`（1–8）。省略参数时使用原图的完整通道数。减少通道会裁掉编号超出配置的端子并重新排版，保留其余端子的 ID、通信和供电接口。若网络仍引用已裁掉的端子，校验会报错。当前范围来自参考模板，其他实体型号的管脚以其资料为准。

中盛继电器模块和电压采集模块的直流输入统一引用 `DC+` 和 `DC-`。旧工程仍可使用中盛继电器的 `V+` 和 `V-` 引用。

用 `python wiring.py part 中盛8通道电压采集模块@1 --parameters '{"channels":4}'` 查询指定通道数的可用端子。参见 [参数样例](projects/zhongsheng-voltage/project-channels.json)及其[出图](projects/zhongsheng-voltage/wiring-channels.svg)。

`行程开关@1` 可由 `parameters.count` 生成 1–9 只行程开关，由 `parameters.start` 指定起始编号，默认从 S1 开始。每只开关提供独立的 1、2、3 端子，例如 `switches.S2-2`。在工程 `nets` 中填写所需接线，参见 [S2、S3 串接样例](projects/limit-switch/project.json)。

`中间继电器` 可用 `parameters.channels` 设置 1–9 组转换触点，默认两组。每组有 `COMn`、`NOn`、`NCn`，所有组共用 `COIL-A` 和 `COIL-B`。参考[四组触点工程](projects/中间继电器/project.json)和[生成图](projects/中间继电器/wiring.svg)。

[`开关电源@1`](library/开关电源/1/part.json) 提供 `L`、`N`、`PE`、`+V`、`-V`。可用实例的 `name` 和 `note` 显示设备名称和输出规格。[气阀控制工程](projects/air-valve-control/project.json) 展示 `气阀控制@1` 的三个外部端子和四个可配置的内部开关位号。

常用电路元件也使用 `library/` 下的模板。电阻、功率电阻、保险丝、二极管、发光二极管、电容、极性电容、电感、常开按钮和常闭按钮可直接加入工程。规格写入实例的 `note`，例如 `20 Ω / 100 W`。参见[示例工程](projects/常用电路元件/project.json)、[继电器与功率电阻接线](projects/常用电路元件/wiring-p01.svg)和[元件符号一览](projects/常用电路元件/wiring-p02.svg)。

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
