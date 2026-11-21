# 工程、网络与出图

## 工程结构

- 工程 JSON 使用 `version: 1`。通过 `sheets` 声明图页的 `id` 和 `title`，通过 `instances[].sheet` 给实例指定图页。同页按可用空间自动排版和换行，空间不足时调整图页分配。
- `instances[].template` 填 `型号@版本`。`id` 是供网表使用的稳定键，`reference` 是图上显示的位号，两者在工程内各自唯一。`name` 可覆盖模板默认名称，`note` 可覆盖默认小字。模板支持时，`station: "02"` 将以小字显示并保留前导零。`parameters` 采用 `part` 给出的字段定义。
- `view` 填 `symbol` 或 `photo`，以模板支持的视图为准。为实例只选一种主视图。照片作为同页独立说明时，用 `sheets[].photo` 填 `file`、`caption`、`marks`。`file` 相对于工程 JSON，`marks[].endpoint` 为 `实例ID.端子ID`，`marks[].at` 为照片内 0–1 的相对坐标。

## 从参考图录入网络

1. 先核对所有模板和参数下有效的端子。逐条追踪参考图上的引线、交点以及同名标签，用 `实例ID.端子ID` 记录端点。确认发送和接收的两端、正负极、地线区分以及原图留空的端子。
2. 每个网络在 `nets` 中用唯一的 `name` 和完整的 `members` 数组定义。保持已有网名，包括中文、空格和加减号。同名网跨页复用同一个网络定义，出图时每个相接端子显示同一网名。
3. 普通网络至少有两个工程内端点，每个端点只加入一个网络。若参考图给出了外部网名，但工程只覆盖其中一个已知端点，可使用 `external: true`。空接脚保留在模板中，工程里不为其创建网络。
4. 从图纸修改已有工程时，保留已经核对的端点与网名。改变通道数后，重新查询该实例的端子，并根据已确认的接线调整受影响的网络。

下列例子中的 `adapter` 是实例 ID，`TXD` 是配件端子 ID。完整工程可参考 [USB 转 TTL](../../../examples/usb-ttl/project.json)。

```json
{
  "name": "继电器1-RXD",
  "members": ["adapter.TXD"],
  "external": true
}
```

## 检查生成结果

```sh
python wiring.py validate 工程.json
python wiring.py render 工程.json 接线图.svg
python wiring.py render 工程.json 接线图.pdf
```

- `validate` 返回图页数以及每个网络的端点清单，端点以 `位号.端子ID` 显示。与参考图逐项核对端子和网名。导出以用户要的格式为准，SVG 多页工程用 `-p01.svg`、`-p02.svg` 等逐页输出。
- 打开生成图，检查端口方向、引线的真实起点、图上的网络标签与空接情况。实物视图核对引线是否指向照片上的接口中心。模块名称和下方小字应相对图形居中，多模块同页时检查标题、模块与网名留有足够空间。
- 若画面排版不足，改配件分组或项目分页，必要时修改模板的端子布置，再重新生成。把资料未能确认的连接或端子单独列给用户核对。

可从 [灯板同页工程](../../../examples/lamp/project-one-page.json) 验证符号与实物视图混排，从[灯板跨页工程](../../../examples/lamp/project.json)核对跨页同网名，从[参数化模块工程](../../../examples/zhongsheng-voltage/project-channels.json)验证站号和通道数，从[行程开关工程](../../../examples/limit-switch/project.json)验证重复开关的独立端子与命名网络。
