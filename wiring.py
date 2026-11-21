from __future__ import annotations

import argparse
from copy import deepcopy
from dataclasses import dataclass
import json
from pathlib import Path
import re
import sys
import tempfile
import xml.etree.ElementTree as ET

import pymupdf as pdf


LIBRARY = Path(__file__).resolve().parent / "library"
# Keep projects saved with the original template IDs loadable.
LEGACY_TEMPLATE_IDS = {
    "1bxrr16": "继电器板-1BXRR16",
    "air-valve-control": "气阀控制",
    "limit-switch": "行程开关",
    "rs485-rs232": "串口转接器-RS485转RS232",
    "switching-power-supply": "开关电源",
    "usb-ttl": "USB转TTL模块",
    "zhongsheng-relay-16in16out": "中盛16入16出继电器模块",
    "zhongsheng-voltage-8ch": "中盛8通道电压采集模块",
}
TOKEN = re.compile(r"[a-zA-Z0-9_-]+\Z")
TEMPLATE_TOKEN = re.compile(r"[\w-]+\Z")
PORT_TOKEN = re.compile(r"[a-zA-Z0-9_+/-]+\Z")
COLOR_TOKEN = re.compile(r"#[0-9a-fA-F]{6}\Z")
DEFAULT_WIRING_COLOR = "#1c5c94"
INK = (0.09, 0.16, 0.23)
MUTED = (0.31, 0.39, 0.47)
RULE = (0.75, 0.80, 0.84)
PAGE_ACCENT = (0.11, 0.36, 0.58)
PAPER = (0.97, 0.98, 0.99)
WHITE = (1, 1, 1)
SYMBOL_TEXT_COLORS = {
    "reference": "#172b3c",
    "description": "#465968",
    "emergency": "#a1273b",
}
SIDES = ("left", "right", "top", "bottom")
FIELDS = ("name", "reference", "note")
MODULE_GAP = 12
SVG_NAMESPACE = "http://www.w3.org/2000/svg"
XLINK_NAMESPACE = "http://www.w3.org/1999/xlink"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def indexed(rows, key, subject, valid=TOKEN.fullmatch):
    result = {}
    for row in rows:
        ident = row[key]
        require(isinstance(ident, str) and valid(ident), f"{subject} {ident!r} 标识无效")
        require(ident not in result, f"{subject} {ident} 重复")
        result[ident] = row
    return result


def local_file(folder, name):
    root = folder.resolve()
    path = (root / name).resolve()
    require(path.is_relative_to(root) and path.is_file(), f"资源应是工程内的文件 {name}")
    return path


@dataclass
class Template:
    public: dict
    asset: Path | bytes
    size: tuple[float, float]
    anchors: dict[str, tuple[float, float, str]]
    view: str


def channel_symbol(layout, counts):
    w, pitch, first_y = layout["width"], layout["pitch"], layout["first_y"]
    gap = 7
    pins, dividers = [], []
    for side, sections in layout["sections"].items():
        y = first_y
        for section in sections:
            groups = [section["pins"]]
            if "parameter" in section:
                count = counts[section["parameter"]]
                half = (count + 1) // 2
                if section.get("half") == "upper":
                    numbers = range(half + 1, count + 1)
                elif section.get("half") == "lower":
                    numbers = range(half, 0, -1)
                else:
                    numbers = [n for n in section["numbers"] if n <= count]
                groups = [[pin.format(n=n) for pin in section["pins"]] for n in numbers]
            if not groups:
                continue
            if y != first_y:
                dividers.append((side, y - (pitch - gap) / 2, True))
                y += gap
            for index, group in enumerate(groups):
                if index and section.get("separate_channels"):
                    dividers.append((side, y - pitch / 2, False))
                for pin in group:
                    pins.append((side, pin, y))
                    y += pitch
            for pin in section.get("trailing", []):
                pins.append((side, pin, y))
                y += pitch

    h = max(y for _, _, y in pins) + 6
    root = ET.Element("svg", xmlns=SVG_NAMESPACE, viewBox=f"0 0 {w:g} {h:g}")
    ET.SubElement(root, "rect", x="17", y="1", width=f"{w-34:g}", height=f"{h-2:g}",
                  rx="5", fill="#f2f7fa", stroke="#6a879b")

    def line(x1, y1, x2, y2, stroke, weight):
        ET.SubElement(root, "line", x1=f"{x1:g}", y1=f"{y1:g}", x2=f"{x2:g}", y2=f"{y2:g}",
                      stroke=stroke, **{"stroke-width": weight})

    middle = w / 2
    line(middle, 5, middle, h - 5, "#c7d7e1", ".6")
    for side, y, section in dividers:
        start, end = (18, middle) if side == "left" else (middle, w - 18)
        line(start, y, end, y, "#a9bdca" if section else "#d4e1e9", ".85" if section else ".55")
    for side, pin, y in pins:
        x, edge, tx, align = (2, 17, 24, "start") if side == "left" else (w-2, w-17, w-24, "end")
        line(x, y, edge, y, "#6a879b", ".8")
        ET.SubElement(root, "circle", cx=f"{x:g}", cy=f"{y:g}", r="1.65", fill="#fff", stroke="#285b7d",
                      **{"stroke-width": ".7", "data-port": pin, "data-side": side})
        label = ET.SubElement(root, "text", x=f"{tx:g}", y=f"{y+2.65:g}",
                             **{"text-anchor": align, "font-family": "DejaVu Sans, sans-serif", "font-size": "8.8"})
        label.text = pin
    return ET.tostring(root, encoding="utf-8")


def shift_symbol_y(element, offset):
    for node in element.iter():
        for coord in ("y", "y1", "y2", "cy"):
            if coord in node.attrib:
                node.set(coord, f"{float(node.get(coord)) + offset:g}")


def repeat_switch_symbol(symbol, ports, count, start):
    prototype = ET.parse(symbol).getroot()
    _, _, width, height = map(float, prototype.attrib["viewBox"].split())
    root = ET.Element(prototype.tag, viewBox=f"0 0 {width:g} {height * count:g}")
    expanded = {}
    for index in range(count):
        label = f"S{start + index}"
        offset = index * height
        for original in prototype:
            element = deepcopy(original)
            shift_symbol_y(element, offset)
            root.append(element)
            for node in element.iter():
                if (terminal := node.get("data-port")) is not None:
                    _, contact = terminal.split("-", 1)
                    pin = f"{label}-{contact}"
                    node.set("data-port", pin)
                    expanded[pin] = {**ports[terminal], "id": pin}
                if node.get("data-switch-label") is not None:
                    node.text = label
    ET.register_namespace("", SVG_NAMESPACE)
    return ET.tostring(root, encoding="utf-8"), expanded


def repeat_relay_symbol(symbol, ports, channels, pitch):
    root = ET.parse(symbol).getroot()
    _, _, width, height = map(float, root.attrib["viewBox"].split())
    rows = [node for node in root if node.get("data-relay-channel") is not None]
    coil = next((node for node in root if node.get("data-relay-coil") is not None), None)
    linkage = next((node for node in root if node.get("data-relay-linkage") is not None), None)
    outline = root.find(f"{{{SVG_NAMESPACE}}}rect")
    require(rows and rows[0].get("data-relay-channel") == "1" and all(node is not None for node in (coil, linkage, outline)),
            "继电器符号须包含第 1 组触点、线圈、联动线和外框")
    prototype = rows[0]
    row_index = list(root).index(prototype)
    delta = (channels - len(rows)) * pitch
    root.set("viewBox", f"0 0 {width:g} {height + delta:g}")
    outline.set("height", f"{float(outline.attrib['height']) + delta:g}")
    link_x = linkage.attrib["data-x"]
    link_top = int(linkage.attrib["data-top"])
    link_bottom = int(linkage.attrib["data-bottom"]) + delta
    linkage.set("d", " ".join(f"M {link_x} {y} V {min(y + 3, link_bottom)}"
                              for y in range(link_top, link_bottom, 6)))
    shift_symbol_y(coil, delta)
    expanded = {pin: ports[pin] for node in coil.iter() if (pin := node.get("data-port")) is not None}
    for row in rows:
        root.remove(row)
    for channel in range(1, channels + 1):
        row = deepcopy(prototype)
        row.set("data-relay-channel", str(channel))
        shift_symbol_y(row, (channel - 1) * pitch)
        labels = {f"{contact}1": f"{contact}{channel}" for contact in ("COM", "NO", "NC")}
        for node in row.iter():
            if (pin := node.get("data-port")) is not None:
                new_pin = labels[pin]
                node.set("data-port", new_pin)
                expanded[new_pin] = {**ports[pin], "id": new_pin, "name": f"第 {channel} 组 {pin[:-1]}"}
            if node.text in labels:
                node.text = labels[node.text]
        root.insert(row_index + channel - 1, row)
    ET.register_namespace("", SVG_NAMESPACE)
    return ET.tostring(root, encoding="utf-8"), expanded


def label_symbol(symbol, formats, parameters):
    root = ET.parse(symbol).getroot()
    labels = {key: pattern.format(**parameters) for key, pattern in formats.items()}
    require(len(set(labels.values())) == len(labels), "模块内部的开关位号不能重复")
    for node in root.iter():
        if (key := node.get("data-parameter-label")) is not None:
            node.text = labels.pop(key)
    require(not labels, "模块图形缺少参数文字位置")
    ET.register_namespace("", SVG_NAMESPACE)
    return ET.tostring(root, encoding="utf-8")


def style_symbol(root, ports, pin_artwork, identity, internal_color):
    for node in pin_artwork:
        node.set("stroke", internal_color)
    circuit_paint = {"wire": "stroke", "junction": "fill"}
    for node in root.iter():
        if (role := node.get("data-circuit-role")) is not None:
            require(role in circuit_paint, f"{identity} 的电路图形角色无效 {role!r}")
            node.set(circuit_paint[role], internal_color)
    palette = {**SYMBOL_TEXT_COLORS, "terminal": internal_color, "interface": internal_color}
    for node in root.iter(f"{{{SVG_NAMESPACE}}}text"):
        role = node.get("data-text-role")
        if role is None and (node.text or "").strip() in ports:
            role = "terminal"
        if role is None:
            continue
        require(role in palette, f"{identity} 的文字角色无效 {role!r}")
        node.set("fill", palette[role])
    ET.register_namespace("", SVG_NAMESPACE)
    return ET.tostring(root, encoding="utf-8")


def load_template(identity, view=None, parameters=None, *, internal_color=DEFAULT_WIRING_COLOR):
    model, sep, version = identity.partition("@")
    require(TEMPLATE_TOKEN.fullmatch(model) and (not sep or version.isdecimal()),
            f"模板引用应为 型号 或 型号@版本 {identity!r}")
    model_dir = LIBRARY / model
    if model in LEGACY_TEMPLATE_IDS and not model_dir.is_dir():
        model = LEGACY_TEMPLATE_IDS[model]
        model_dir = LIBRARY / model
    if not sep:
        versions = [path.parent.name for path in model_dir.glob("*/part.json")
                    if path.parent.name.isdecimal() and path.is_file()]
        require(versions, f"未找到模板 {model!r} 的任何版本")
        version = max(versions, key=lambda name: (int(name), name))
        identity = f"{model}@{version}"
    folder = model_dir / version
    public = read_json(folder / "part.json")
    require(public["id"] == model and str(public["version"]) == version, f"模板 ID 或版本与目录不一致 {identity}")
    ports = indexed(public["ports"], "id", "端子", PORT_TOKEN.fullmatch)
    specs = public.get("parameters", {})
    parameters = {} if parameters is None else parameters
    require(isinstance(parameters, dict) and parameters.keys() <= specs.keys(), f"{identity} 的模板参数无效")
    counts = {key: parameters.get(key, spec["default"]) for key, spec in specs.items()}
    for key, value in counts.items():
        spec = specs[key]
        require(type(value) is int and spec["min"] <= value <= spec["max"],
                f"{identity} 的 {key} 应为 {spec['min']}–{spec['max']} 的整数")
    variant = any(value != specs[key]["default"] for key, value in counts.items())
    symbol = folder / "symbol.svg"
    photo = folder / "photo.json"
    public["views"] = [name for name, path in (("symbol", symbol), ("photo", photo)) if path.is_file()]
    require(public["views"], f"配件缺少图面模板 {identity}")
    view = public["views"][0] if view is None else view
    require(view in public["views"], f"配件 {identity} 不支持图面 {view!r}")
    if view == "photo":
        require(not variant, f"{identity} 的实物图面不支持重排通道数")
        info = read_json(photo)
        asset = local_file(folder, info["file"])
        image = pdf.Pixmap(asset)
        width, height = 360, 360 * image.height / image.width
        anchors = {}
        for port, point in info["ports"].items():
            x, y = point["at"]
            side = point["side"]
            require(side in SIDES and all(isinstance(v, (int, float)) and 0 <= v <= 1 for v in (x, y)),
                    f"实物端子锚点无效 {identity}.{port}")
            anchors[port] = (x * width, y * height, side)
        require(anchors.keys() == ports.keys(), f"实物模板端子与锚点不一致 {identity}")
        return Template(public, asset, (width, height), anchors, view)

    if variant:
        layout = read_json(folder / "channel-layout.json")
        repeat_switches = layout.get("repeat_switches", False)
        if repeat_switches:
            asset, ports = repeat_switch_symbol(symbol, ports, counts["count"], counts["start"])
        elif "relay_pitch" in layout:
            asset, ports = repeat_relay_symbol(symbol, ports, counts["channels"], layout["relay_pitch"])
        elif "labels" in layout:
            asset = label_symbol(symbol, layout["labels"], counts)
        else:
            asset = channel_symbol(layout, counts)
        root = ET.fromstring(asset)
        public["model"] = layout["model"].format(**counts)
    else:
        root = ET.parse(symbol).getroot()
    x, y, width, height = map(float, root.attrib["viewBox"].split())
    require(x == y == 0 and 0 < width <= 10000 and 0 < height <= 10000, f"模板尺寸无效 {identity}")
    leads_at = {}
    for line in root.iter(f"{{{SVG_NAMESPACE}}}line"):
        for end in (1, 2):
            point = (float(line.attrib[f"x{end}"]), float(line.attrib[f"y{end}"]))
            leads_at.setdefault(point, []).append(line)
    anchors = {}
    pin_artwork = set()
    for node in root.iter():
        if (port := node.get("data-port")) is not None:
            pin_x, pin_y = float(node.attrib["cx"]), float(node.attrib["cy"])
            side = node.attrib["data-side"]
            require(port not in anchors and side in SIDES and 0 <= pin_x <= width and 0 <= pin_y <= height,
                    f"模板锚点无效 {identity}.{port}")
            require((pin_x, pin_y) in leads_at, f"模板引脚线与锚点有断口 {identity}.{port}")
            anchors[port] = (pin_x, pin_y, side)
            pin_artwork.add(node)
            pin_artwork.update(leads_at[(pin_x, pin_y)])
    if variant:
        require(anchors.keys() <= ports.keys(), f"图形使用了模板未定义的端子 {identity}")
        public["ports"] = [ports[pin] for pin in anchors]
    else:
        require(anchors.keys() == ports.keys(), f"模板端子与锚点不一致 {identity}")
    return Template(public, style_symbol(root, ports, pin_artwork, identity, internal_color), (width, height), anchors, view)


def runs(value, mono):
    for run in re.findall(r"[\x00-\x7F]+|[^\x00-\x7F]+", value):
        name = "china-s" if not run.isascii() else ("cour" if mono else "helv")
        yield run, name


def width(value, size, mono=False):
    return sum(pdf.get_text_length(run, fontname=name, fontsize=size) for run, name in runs(value, mono))


def text(page, x, y, value, size, *, color=INK, mono=False, right=False):
    if right:
        x -= width(value, size, mono)
    for run, name in runs(value, mono):
        page.insert_text((x, y), run, fontname=name, fontsize=size, color=color)
        x += pdf.get_text_length(run, fontname=name, fontsize=size)


def project_data(path):
    data = read_json(path)
    require(data["version"] == 1, "工程版本应为 1")
    colors = data.get("colors", {})
    require(isinstance(colors, dict) and colors.keys() <= {"internal", "external"},
            "工程 colors 应为包含 internal 或 external 的对象")
    for name, value in colors.items():
        require(isinstance(value, str) and COLOR_TOKEN.fullmatch(value), f"工程 colors.{name} 应使用 #RRGGBB 格式")
    data["colors"] = {name: colors.get(name, DEFAULT_WIRING_COLOR) for name in ("internal", "external")}
    sheets = indexed(data["sheets"], "id", "图页")
    instances = indexed(data["instances"], "id", "配件实例")
    require(sheets and instances, "工程至少需要一页和一个配件实例")
    indexed(instances.values(), "reference", "位号")
    choices = {ident: (device["template"], device.get("view"), json.dumps(device.get("parameters", {}), sort_keys=True))
               for ident, device in instances.items()}
    loaded = {(model, view, params): load_template(model, view, json.loads(params), internal_color=data["colors"]["internal"])
              for model, view, params in set(choices.values())}
    templates = {ident: loaded[choice] for ident, choice in choices.items()}
    for ident, device in instances.items():
        require(device["sheet"] in sheets, f"{device['reference']} 引用了不存在的图页")
        require(all(isinstance(device.get(field, ""), str) for field in FIELDS), f"{device['reference']} 的文本字段无效")
        if "station" in device:
            station = device["station"]
            require("station" in templates[ident].public["fields"], f"{device['reference']} 的模板不支持站号")
            require(isinstance(station, str) and station.isprintable() and station == station.strip(),
                    f"{device['reference']} 的站号应为文本")
    endpoint_names = {f"{ident}.{pin}": f"{device['reference']}.{pin}"
                      for ident, device in instances.items() for pin in templates[ident].anchors}
    nets = indexed(data["nets"], "name", "网络",
                   lambda name: bool(name) and name.isprintable() and name == name.strip())
    members = {}
    for name, net in nets.items():
        external = net.get("external", False)
        require(isinstance(external, bool), f"网络 {name} 的 external 应为布尔值")
        require(isinstance(net["members"], list) and len(net["members"]) >= (1 if external else 2),
                f"网络 {name} 至少需要{'一个' if external else '两个'}端子")
        for endpoint in net["members"]:
            require(endpoint in endpoint_names and endpoint not in members, f"端子不存在或已加入网络 {endpoint}")
            members[endpoint] = name
    return data, sheets, instances, templates, nets, members, endpoint_names


def place_rows(preferred, low, high, spacing):
    require(len(preferred) <= 1 or high - low >= (len(preferred) - 1) * spacing, "网络标签空间不足，请分图页")
    rows = []
    for index, pos in enumerate(preferred):
        floor = rows[-1] + spacing if rows else low
        ceiling = high - (len(preferred) - index - 1) * spacing
        rows.append(min(max(floor, pos), ceiling))
    return rows


@dataclass
class DeviceBlock:
    template: Template
    heading: str
    note: str
    grouped: dict
    connected_ports: tuple[str, ...]
    insets: tuple[float, float, float, float]
    net_font: int
    net_spacing: int

    def horizontal_space(self, scale):
        left, _, right, _ = self.insets
        half_symbol = self.template.size[0] * scale / 2
        half_caption = max(width(self.heading, 14), width(self.note, 9)) / 2 + 10
        return max(half_symbol + left, half_caption), max(half_symbol + right, half_caption)

    def size(self, scale):
        _, top, _, bottom = self.insets
        h = self.template.size[1]
        label_rows = max(len(self.grouped["left"]), len(self.grouped["right"]))
        return (sum(self.horizontal_space(scale)),
                max(h * scale + top + bottom, 60 + max(0, label_rows - 1) * self.net_spacing))


def device_block(device, template, tags):
    heading = f"{device['reference']}   {device.get('name') or template.public['name']}"
    note = device.get("note") or template.public["model"]
    if device.get("station"):
        note += f" · 站号 {device['station']}"
    grouped = {side: [] for side in SIDES}
    for pin, tag in tags.items():
        anchor = template.anchors[pin]
        grouped[anchor[2]].append((anchor, tag))
    compact = bool(template.public.get("parameters")) or max(map(len, grouped.values())) >= 16
    net_font, net_spacing = (8, 9) if compact else (10, 15)
    padding = 12 if template.view == "photo" else 6
    reserve = {side: max((width(name, net_font, True) + 32 for _, name in grouped[side]), default=padding)
               for side in SIDES}
    insets = (reserve["left"], 38 + padding + len(grouped["top"]) * net_spacing,
              max(reserve[side] for side in SIDES[1:]), padding + len(grouped["bottom"]) * net_spacing)
    return DeviceBlock(template, heading, note, grouped, tuple(tags), insets, net_font, net_spacing)


def pack_shelves(sizes, available_width, available_height):
    """Place consecutive blocks on a row, opening a new row when it fills."""
    positions = []
    x = y = row_height = 0
    for w, h in sizes:
        if x and x + w > available_width:
            x, y, row_height = 0, y + row_height + MODULE_GAP, 0
        if w > available_width or y + h > available_height:
            return None
        positions.append((x, y))
        x += w + MODULE_GAP
        row_height = max(row_height, h)
    return positions


def layout_devices(blocks, page_width, page_height, sheet):
    for factor in (1, .9, .8, .7, 0):
        scales = [max(.55, .75 * factor) if block.template.view == "photo"
                  else max(block.template.public["min_scale"], .95 * factor) for block in blocks]
        sizes = [block.size(scale) for block, scale in zip(blocks, scales, strict=True)]
        if (positions := pack_shelves(sizes, page_width, page_height)) is not None:
            return zip(positions, blocks, scales, strict=True)
        if (columns := pack_shelves([(h, w) for w, h in sizes], page_height, page_width)) is not None:
            return zip(((y, x) for x, y in columns), blocks, scales, strict=True)
    raise ValueError(f"图页 {sheet} 的模块及网络标签空间不足，请将部分模块移至其他图页")


def draw_device(page, x0, y0, block, scale, colors):
    dw, dh = block.size(scale)
    x1, y1 = x0 + dw, y0 + dh
    center = x0 + block.horizontal_space(scale)[0]
    text(page, center - width(block.heading, 14) / 2, y0 + 19, block.heading, 14)
    text(page, center - width(block.note, 9) / 2, y0 + 32, block.note, 9, color=MUTED)
    page.draw_line((x0 + 10, y0 + 38), (x1 - 10, y0 + 38), color=RULE, width=.5)

    template = block.template
    _, top, _, _ = block.insets
    w, h = template.size
    ox, oy = center - w * scale / 2, y0 + top
    box = pdf.Rect(ox, oy, ox + w * scale, oy + h * scale)
    real_photo = template.view == "photo"
    if real_photo:
        page.insert_image(box, filename=template.asset)
    else:
        with pdf.open(stream=template.asset, filetype="svg") as symbol:
            with pdf.open(stream=symbol.convert_to_pdf(), filetype="pdf") as vector:
                page.show_pdf_page(box, vector, 0, keep_proportion=False)

    for side, entries in block.grouped.items():
        is_vertical = side in ("top", "bottom")
        entries.sort(key=lambda item: item[0][0 if is_vertical else 1])
        preferred = [oy + p[1] * scale for p, _ in entries]
        rows = place_rows(preferred, y0 + 50, y1 - 10, block.net_spacing) if not is_vertical else []
        for index, ((x, y, _), name) in enumerate(entries):
            a, b = ox + x * scale, oy + y * scale
            if is_vertical:
                step = len(entries) - index - 1
                offset = 10 + step * block.net_spacing
                row = box.y0 - offset if side == "top" else box.y1 + offset
                elbow = (a, row)
            else:
                row = rows[index]
                elbow = (a + (-6 if side == "left" else 6), b)
            rail = box.x0 - 20 if side == "left" else box.x1 + 20
            page.draw_polyline([(a, b), elbow, (rail, row)], color=colors["external"], width=1.05)
            tx = rail + (-5 if side == "left" else 5)
            text(page, tx, row + 1, name, block.net_font, color=colors["external"], mono=True, right=side == "left")

    if real_photo:
        marks = []
        for pin in block.connected_ports:
            x, y, _ = template.anchors[pin]
            marks.append((pin, ox + x * scale, oy + y * scale))
        draw_photo_marks(page, box, marks, color=colors["internal"], compact=True)


def draw_photo(page, frame, photo, path, endpoint_names, color):
    x0, y0, x1, y1 = frame
    text(page, x0 + 10, y0 + 23, "实物接口", 16)
    image = pdf.Pixmap(local_file(path.parent, photo["file"]))
    ratio = min((x1 - x0 - 20) / image.width, (y1 - y0 - 85) / image.height)
    left, top = x0 + 10, y0 + 50
    box = pdf.Rect(left, top, left + image.width * ratio, top + image.height * ratio)
    page.insert_image(box, pixmap=image)
    require(width(photo["caption"], 9) < x1 - x0 - 20, "照片说明过长")
    text(page, left, box.y1 + 18, photo["caption"], 9, color=MUTED)
    marks = []
    for mark in photo["marks"]:
        endpoint = mark["endpoint"]
        rx, ry = mark["at"]
        require(endpoint in endpoint_names, f"照片端子引用无效 {endpoint}")
        require(all(isinstance(value, (int, float)) and 0 <= value <= 1 for value in (rx, ry)), f"照片标记坐标无效 {endpoint}")
        x, y = box.x0 + rx * box.width, box.y0 + ry * box.height
        marks.append((endpoint_names[endpoint], x, y))
    draw_photo_marks(page, box, marks, color=color)


def draw_photo_marks(page, box, marks, *, color, compact=False):
    font, height, radius, inset, padding = (8, 12, 3, 16, 4) if compact else (10, 17, 5, 31, 6)
    columns = {1: [], -1: []}
    for name, x, y in marks:
        tag_w = width(name, font, True) + 2 * padding
        direction = 1 if x + inset + tag_w <= box.x1 else -1
        columns[direction].append((x, y, name, tag_w))

    label_boxes = []
    for direction, entries in columns.items():
        entries.sort(key=lambda entry: entry[1])
        low, high = box.y0 + 1, box.y1 - height - 1
        preferred = [y - height // 2 for _, y, _, _ in entries]
        rows = place_rows(preferred, low, high, height + 1) if compact else [max(low, min(y, high)) for y in preferred]
        for (x, y, name, tag_w), ty in zip(entries, rows, strict=True):
            tx = x + inset if direction == 1 else x - inset - tag_w
            tag = pdf.Rect(tx, ty, tx + tag_w, ty + height)
            require(box.contains(tag) and not any(tag.intersects(other) for other in label_boxes), f"照片标签空间不足 {name}")
            label_boxes.append(tag)
            page.draw_circle((x, y), radius, color=color, width=1.15 if compact else 1.6)
            page.draw_line((x + radius * direction, y), (tx if direction == 1 else tx + tag_w, ty + height // 2), color=color, width=.75 if compact else 1)
            page.draw_rect(tag, color=color, fill=WHITE, width=.65)
            text(page, tx + padding, ty + (height - 3 if compact else 12), name, font, color=color, mono=True)


def compose(path):
    data, sheets, devices, templates, nets, members, endpoint_names = project_data(path)
    colors = {role: tuple(channel / 255 for channel in bytes.fromhex(value[1:]))
              for role, value in data["colors"].items()}
    sheet_order = {sheet: i for i, sheet in enumerate(sheets)}
    document = pdf.open()
    try:
        for _ in sheets:
            document.new_page(width=842, height=595)
        document.set_metadata({"title": data["title"], "subject": data.get("source", "")})
        require(width(data["title"], 11) < 610 and width(data.get("source", ""), 9) < 778, "工程标题或来源说明过长")
        for sheet, number in sheet_order.items():
            page = document[number]
            require(width(sheets[sheet]["title"], 27) < 610, f"图页 {sheet} 的标题过长")
            page.draw_rect(page.rect, color=None, fill=WHITE)
            page.draw_rect((32, 32, 36, 78), color=None, fill=PAGE_ACCENT)
            text(page, 48, 41, data["title"], 11, color=MUTED)
            text(page, 48, 72, sheets[sheet]["title"], 27)
            text(page, 808, 58, f"{sheet} / {len(sheets):02d}", 13, color=PAGE_ACCENT, mono=True, right=True)
            page.draw_line((32, 91), (810, 91), color=RULE, width=.65)
            text(page, 32, 570, data.get("source", ""), 9, color=MUTED)
            page.draw_line((32, 550), (810, 550), color=RULE, width=.65)
            rows = [device for device in devices.values() if device["sheet"] == sheet]
            photo = sheets[sheet].get("photo")
            require(rows, f"图页 {sheet} 缺少配件")
            # shortcut: one photo column per sheet, extend when real drawings need several photos side by side.
            edge = 394 if photo else 810
            blocks = []
            for device in rows:
                ident = device["id"]
                template = templates[ident]
                tags = {pin: name for pin in template.anchors
                        if (name := members.get(f"{ident}.{pin}")) is not None}
                blocks.append(device_block(device, template, tags))
            for (x, y), block, scale in layout_devices(blocks, edge - 32, 430, sheet):
                draw_device(page, 32 + x, 108 + y, block, scale, colors)
            if photo:
                page.draw_rect((410, 108, 810, 414), color=RULE, fill=PAPER, width=.65)
                draw_photo(page, (410, 108, 810, 414), photo, path, endpoint_names, colors["internal"])
        return document, {name: [endpoint_names[p] for p in net["members"]] for name, net in nets.items()}
    except Exception:
        document.close()
        raise


def svg_outputs(document, output):
    paths = [output.with_stem(f"{output.stem}-p{i + 1:02d}") if len(document) > 1 else output
             for i in range(len(document))]
    ET.register_namespace("", SVG_NAMESPACE)
    ET.register_namespace("xlink", XLINK_NAMESPACE)
    ns = f"{{{SVG_NAMESPACE}}}"
    outputs = {}
    for page, path in zip(document, paths, strict=True):
        root = ET.fromstring(page.get_svg_image(text_as_path=True))
        title = ET.Element(f"{ns}title")
        title.text = f"{document.metadata['title']} · 第 {page.number + 1} 页"
        root.insert(0, title)
        description = ET.Element(f"{ns}desc")
        description.text = page.get_text()
        root.insert(1, description)
        outputs[path] = ET.tostring(root, encoding="utf-8", xml_declaration=True)
    return outputs


def write_outputs(outputs):
    parent = next(iter(outputs)).parent
    with tempfile.TemporaryDirectory(dir=parent) as temp:
        for path, content in outputs.items():
            (Path(temp) / path.name).write_bytes(content)
        for path in outputs:
            (Path(temp) / path.name).replace(path)


def main():
    parser = argparse.ArgumentParser(description="测试治具接线图")
    commands = parser.add_subparsers(dest="command", required=True)
    part = commands.add_parser("part")
    part.add_argument("identity", nargs="?")
    part.add_argument("--parameters", type=json.loads)
    commands.add_parser("validate").add_argument("project", type=Path)
    render = commands.add_parser("render")
    render.add_argument("project", type=Path)
    render.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        if args.command == "part":
            if args.identity:
                fields = load_template(args.identity, parameters=args.parameters).public.copy()
                fields.pop("min_scale", None)
                result = fields
            else:
                require(args.parameters is None, "查询参数时请指定配件模板")
                result = [f"{p.parent.parent.name}@{p.parent.name}" for p in sorted(LIBRARY.glob("*/*/part.json"))]
        else:
            if args.command == "render":
                require(args.output.suffix.lower() in (".pdf", ".svg") and args.output.resolve() != args.project.resolve(),
                        "输出路径应是单独的 PDF 或 SVG 文件")
            document, nets = compose(args.project)
            with document:
                result = {"pages": len(document), "nets": nets}
                if args.command == "render":
                    if args.output.suffix.lower() == ".svg":
                        outputs = svg_outputs(document, args.output)
                        result["svg"] = [str(p) for p in outputs]
                    else:
                        outputs = {args.output: document.tobytes(garbage=3, deflate=True)}
                        result["pdf"] = str(args.output)
                    write_outputs(outputs)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (ValueError, KeyError, TypeError, AttributeError, OSError, RuntimeError, ET.ParseError) as error:
        print(f"接线图校验失败：{error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
