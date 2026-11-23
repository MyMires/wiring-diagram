# 测试治具接线图

从器件模板和工程 JSON 生成测试治具接线图。`part` 查询模板和端子，`validate` 检查网表与排版，`render` 输出 SVG 或 PDF。发布记录与对应源码见 [GitHub Releases](https://github.com/MyMires/wiring-diagram/releases)。

## 从源码运行

在包含 `wiring.py` 和 `library/` 的源码工作区运行。源码版使用 Python 3.11+ 和 `pymupdf-lite`。[PDF 引擎](https://pypi.org/project/pymupdf-lite/2.1/)

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

从发行页下载 `wiring.exe`，放到工程目录里即可运行，适用于 Windows 10 和 11 x64。原生版内置出图引擎、中文字体和构建时的器件库。工程 JSON 与它引用的实物照片随工程目录保存。[下载 wiring.exe](https://github.com/MyMires/wiring-diagram/releases/latest/download/wiring.exe)

以下命令在仓库根目录和 EXE 同目录时可直接运行。

```powershell
.\wiring.exe --version
.\wiring.exe part
.\wiring.exe validate .\projects\lamp\project.json
.\wiring.exe render .\projects\lamp\project.json .\wiring.svg
```

修改器件库后可以重建单文件。以下命令适用于装有 Rust 和 MinGW-w64 的 Linux 构建机。产物位于 `native/target/x86_64-pc-windows-gnu/release/wiring.exe`。第三方许可声明随发行下载，也可通过 `wiring.exe --licenses` 查看。

```sh
rustup target add x86_64-pc-windows-gnu
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C target-feature=+crt-static" \
  cargo build --locked --release --manifest-path native/Cargo.toml --target x86_64-pc-windows-gnu
```

## 工具版本

工具版本采用 `主版本.次版本.修订号`。发布时同步更新 `wiring_version.py`、`native/Cargo.toml` 和锁文件。重新构建后，核对源码版和 Windows 版的 `--version`、EXE 文件属性及代表性工程出图。Git 标签 `v<版本号>` 指向已经验证的发布提交。[源码版本](wiring_version.py) · [原生构建配置](native/Cargo.toml)

当前工具读取 `"version": 1` 的工程 JSON。`型号@版本` 选定器件模板版本。[工程格式](skills/test-wiring/references/projects.md) · [模板格式](skills/test-wiring/references/templates.md)

## 器件库

用 `part` 查询当前可用模板。每个版本的 `library/型号/版本/README.md` 介绍用途、接线端子、配置方法和使用限制，附工程实例示例。新增模板或改变接口、参数和视图时，同步更新对应版本的说明。[器件文档约定](skills/test-wiring/references/templates.md#维护器件文档)

## 示例工程

`library/` 存可复用的器件模板，`projects/` 存接线工程和生成图。模板目录使用中文名称，保留实际型号和接口缩写。模板目录名与 `part.json` 的 `id` 一致，工程可用 `"template": "气阀控制"` 选最高数字版本，或用 `"template": "气阀控制@1"` 固定第 1 版。实例的 `name` 可单独设置显示名称。旧工程中的英文模板标识仍可解析。

[`projects/lamp`](projects/lamp/) 包含灯板通信接线摘录和可重新出图的工程。`part` 命令返回可编辑文本字段、端子定义和支持的图面。实例通过 `view` 选择 `symbol` 或 `photo`，两种图面共用端子 ID。SVG 符号和实物照片的相对坐标均在模板内部，网络引线起于所选图面的端子锚点。样例中的 `X1` 使用实物模板，照片上标记已接线的 `A`、`B`。

在 `sheets` 声明图页，并给同页配件填写相同的 `instances[].sheet`。生成器根据模块、标题和网名的尺寸紧凑排版。比如 [`project-one-page.json`](projects/lamp/project-one-page.json) 将继电器板和转接器放在同一页，自动生成 [`wiring-one-page.svg`](projects/lamp/wiring-one-page.svg)。

[`projects/zhongsheng-voltage`](projects/zhongsheng-voltage/) 是 16 入 16 出继电器板和 8 通道电压采集模块的单页接线。网络可使用中文原名。若参考图只给出本工程内的一个端子及外部网名，在该网络设置 `"external": true`，并保留本工程已知的端子。

- 中盛参数化工程展示站号和通道数设置。[工程](projects/zhongsheng-voltage/project-channels.json) · [接线图](projects/zhongsheng-voltage/wiring-channels.svg) · [继电器用法](library/中盛16入16出继电器模块/1/README.md) · [采集模块用法](library/中盛8通道电压采集模块/1/README.md)
- 行程开关与中间继电器工程展示重复开关和多组转换触点。[S2、S3 串接工程](projects/limit-switch/project.json) · [四组触点工程](projects/中间继电器/project.json) · [四组触点接线图](projects/中间继电器/wiring.svg)
- 气阀控制和治具供电工程展示组合线路及其可配置的开关位号。[气阀控制工程](projects/air-valve-control/project.json) · [治具供电工程](projects/治具供电/project.json) · [治具供电接线图](projects/治具供电/wiring.svg)
- 常用电路元件工程展示符号、规格备注和串联接线。[工程](projects/常用电路元件/project.json) · [继电器与功率电阻接线](projects/常用电路元件/wiring-p01.svg) · [元件符号一览](projects/常用电路元件/wiring-p02.svg)

`render` 根据扩展名出图。多页 SVG 使用 `wiring-p01.svg`、`wiring-p02.svg` 等文件名。SVG 文字以矢量轮廓输出，以保持显示一致，原文保存在图的描述元素中。

首版使用 A4 横向、按工程指定图页排版。配件实物模板随实例放置，图页还可通过 `photo` 附一张说明照片。达到可读尺寸下限仍放不下时，校验会指出需要拆分的图页。同一工程中，标有同一网络名的端子属于同一个网络，适用于同页和跨页接线。`validate` 输出每个网络的完整成员。

## 工程配色

在工程 JSON 顶层填写 `colors`。`internal` 控制模块内导线、端点、端子文字和照片接点标记，`external` 控制外部引线和网名。两项都接受 `#RRGGBB`，也都可以单独省略。缺省项使用蓝色 `#1C5C94`，省略整个 `colors` 时两类线路均用该蓝色。接头和普通端子使用同一内部颜色。

[四模块工程](projects/four-modules/project.json) 只指定 `"colors": {"external": "#9A4A16"}`，因此模块内部用默认蓝色，外部用自定义深橙色。气阀控制、UUT 电源、USB 转 TTL 和行程开关实例都指向 `P01`，由工具自动排在同一张 A4 上。[查看生成的 SVG](projects/four-modules/wiring.svg)

```sh
python wiring.py validate projects/four-modules/project.json
python wiring.py render projects/four-modules/project.json projects/four-modules/wiring.svg
```

## 测试接线 Skill

可以单独复制 `skills/test-wiring/` 目录使用。接线时提供可用的工具和工程路径，Skill 按当前工具支持的格式工作。器件用法由各版本目录的文档维护，工具发布和器件更新可按各自节奏进行。[测试接线 Skill](skills/test-wiring/SKILL.md)
