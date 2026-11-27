use std::{
    collections::{HashMap, HashSet},
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageFormat, ImageReader};
use include_dir::{Dir, include_dir};
use serde_json::{Value, json};
use xmltree::{Element, XMLNode};

static LIBRARY: Dir = include_dir!("$CARGO_MANIFEST_DIR/../library");

const LEGACY_NAMES: [(&str, &str); 8] = [
    ("1bxrr16", "继电器板-1BXRR16"),
    ("air-valve-control", "气阀控制"),
    ("limit-switch", "行程开关"),
    ("rs485-rs232", "串口转接器-RS485转RS232"),
    ("switching-power-supply", "开关电源"),
    ("usb-ttl", "USB转TTL模块"),
    ("zhongsheng-relay-16in16out", "中盛16入16出继电器模块"),
    ("zhongsheng-voltage-8ch", "中盛8通道电压采集模块"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

pub const SIDES: [Side; 4] = [Side::Left, Side::Right, Side::Top, Side::Bottom];

impl Side {
    fn parse(value: &str) -> Result<Self> {
        Ok(match value {
            "left" => Self::Left,
            "right" => Self::Right,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            _ => bail!("无效端子方向 {value:?}"),
        })
    }
}

#[derive(Clone)]
pub struct Anchor {
    pub pin: String,
    pub x: f32,
    pub y: f32,
    pub side: Side,
}

pub enum Asset {
    Symbol(String),
    Photograph(Photo),
}

pub struct Template {
    pub public: Value,
    pub asset: Asset,
    pub size: (f32, f32),
    pub anchors: Vec<Anchor>,
}

impl Template {
    pub fn component_anchors(&self) -> Option<[usize; 2]> {
        let [a, b] = self.anchors.as_slice() else {
            return None;
        };
        if !matches!(self.asset, Asset::Symbol(_)) || a.y != b.y {
            return None;
        }
        match (a.side, b.side) {
            (Side::Left, Side::Right) if a.x < b.x => Some([0, 1]),
            (Side::Right, Side::Left) if b.x < a.x => Some([1, 0]),
            _ => None,
        }
    }
}

pub struct Photo {
    pub width: f32,
    pub height: f32,
    pub url: String,
}

impl Photo {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let format = image::guess_format(bytes).context("无法识别实物照片格式")?;
        let (width, height) =
            ImageReader::with_format(Cursor::new(bytes), format).into_dimensions()?;
        let (media, data) = match format {
            ImageFormat::Png => ("image/png", STANDARD.encode(bytes)),
            ImageFormat::Jpeg => ("image/jpeg", STANDARD.encode(bytes)),
            _ => {
                let mut png = Cursor::new(Vec::new());
                image::load_from_memory_with_format(bytes, format)?
                    .write_to(&mut png, ImageFormat::Png)?;
                ("image/png", STANDARD.encode(png.into_inner()))
            }
        };
        Ok(Self {
            width: width as f32,
            height: height as f32,
            url: format!("data:{media};base64,{data}"),
        })
    }
}

pub fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .with_context(|| format!("{field} 应为文本"))
}

pub fn array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value]> {
    value[field]
        .as_array()
        .map(Vec::as_slice)
        .with_context(|| format!("{field} 应为数组"))
}

fn number(value: &Value) -> Result<f32> {
    value
        .as_f64()
        .map(|v| v as f32)
        .filter(|v| v.is_finite())
        .context("坐标应为有限数值")
}

pub fn photo_point(value: &Value) -> Result<(f32, f32)> {
    let coordinates = array(value, "at")?;
    let [x, y] = coordinates else {
        bail!("照片坐标应为两个数值")
    };
    let (x, y) = (number(x)?, number(y)?);
    ensure!(
        (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
        "照片坐标应在 0–1 之间"
    );
    Ok((x, y))
}

fn versions<'a>(dir: &'a Dir<'_>) -> impl Iterator<Item = &'a str> {
    dir.dirs().filter_map(|version| {
        let name = version.path().file_name()?.to_str()?;
        let part = version.path().join("part.json");
        (name.bytes().all(|b| b.is_ascii_digit()) && LIBRARY.get_file(part).is_some())
            .then_some(name)
    })
}

pub fn identities() -> Vec<String> {
    let mut result: Vec<_> = LIBRARY
        .dirs()
        .flat_map(|dir| versions(dir).map(|version| (dir.path(), version)))
        .collect();
    result.sort();
    result
        .into_iter()
        .map(|(path, version)| format!("{}@{version}", path.display()))
        .collect()
}

fn file(path: impl AsRef<Path>) -> Result<&'static [u8]> {
    let path = path.as_ref();
    LIBRARY
        .get_file(path)
        .map(|file| file.contents())
        .with_context(|| format!("模板文件不存在 {}", path.display()))
}

fn read(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_slice(file(path)?)?)
}

fn asset_path(folder: &Path, name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    ensure!(
        path.components()
            .all(|component| matches!(component, std::path::Component::Normal(_))),
        "资源应是模板内的文件 {name}"
    );
    Ok(folder.join(path))
}

fn is_model_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
}

fn is_pin(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_+/-".contains(&b))
}

pub fn load(
    identity: &str,
    view: Option<&str>,
    parameters: Option<&Value>,
    color: &str,
) -> Result<Template> {
    let (model, version) = identity
        .split_once('@')
        .map_or((identity, None), |(m, v)| (m, Some(v)));
    ensure!(
        is_model_token(model)
            && version.is_none_or(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit())),
        "模板引用应为 型号 或 型号@版本 {identity:?}"
    );
    let model = if LIBRARY.get_dir(model).is_some() {
        model
    } else {
        LEGACY_NAMES
            .iter()
            .find_map(|&(old, name)| (old == model).then_some(name))
            .unwrap_or(model)
    };
    let dir = LIBRARY
        .get_dir(model)
        .with_context(|| format!("未找到模板 {model:?} 的任何版本"))?;
    let version = match version {
        Some(version) => version,
        None => versions(dir)
            .max_by_key(|name| {
                let significant = name.trim_start_matches('0');
                (significant.len(), significant, *name)
            })
            .with_context(|| format!("未找到模板 {model:?} 的任何版本"))?,
    };
    let folder = dir.path().join(version);
    let mut public = read(folder.join("part.json"))?;
    ensure!(
        public["id"] == model && public["version"].to_string() == version,
        "模板 ID 或版本与目录不一致 {identity}"
    );
    let mut ports = HashMap::new();
    for port in array(&public, "ports")? {
        let id = string(port, "id")?;
        ensure!(
            is_pin(id) && ports.insert(id.to_owned(), port.clone()).is_none(),
            "端子 {id:?} 无效或重复"
        );
    }

    let mut counts = HashMap::new();
    let mut variant = false;
    let specs = public["parameters"].as_object();
    if let Some(parameters) = parameters.filter(|value| !value.is_null()) {
        let parameters = parameters.as_object().context("模板参数应为 JSON 对象")?;
        ensure!(
            parameters
                .keys()
                .all(|name| specs.is_some_and(|specs| specs.contains_key(name))),
            "{identity} 的模板参数无效"
        );
    }
    for (name, spec) in specs.into_iter().flatten() {
        let default = spec["default"]
            .as_i64()
            .context("模板参数的默认值应为整数")?;
        let min = spec["min"].as_i64().context("模板参数的最小值应为整数")?;
        let max = spec["max"].as_i64().context("模板参数的最大值应为整数")?;
        let value = parameters
            .and_then(|parameters| parameters.get(name))
            .map_or(Some(default), Value::as_i64);
        let value = value
            .filter(|value| (min..=max).contains(value))
            .with_context(|| format!("{identity} 的 {name} 应为 {min}–{max} 的整数"))?;
        variant |= value != default;
        counts.insert(name.clone(), value);
    }

    let symbol = folder.join("symbol.svg");
    let photo = folder.join("photo.json");
    let views: Vec<_> = [("symbol", &symbol), ("photo", &photo)]
        .into_iter()
        .filter_map(|(name, path)| LIBRARY.get_file(path).is_some().then_some(name))
        .collect();
    let view = view
        .or_else(|| views.first().copied())
        .with_context(|| format!("配件缺少图面模板 {identity}"))?;
    ensure!(views.contains(&view), "配件 {identity} 不支持图面 {view:?}");
    public["views"] = json!(views);
    if view == "photo" {
        ensure!(!variant, "{identity} 的实物图面不支持重排通道数");
        let config = read(photo)?;
        let photo = Photo::from_bytes(file(asset_path(&folder, string(&config, "file")?)?)?)?;
        let size = (360.0, 360.0 * photo.height / photo.width);
        let mut anchors = Vec::new();
        for (pin, value) in config["ports"]
            .as_object()
            .context("照片模板缺少端子锚点")?
        {
            let (x, y) = photo_point(value)?;
            anchors.push(Anchor {
                pin: pin.clone(),
                x: x * size.0,
                y: y * size.1,
                side: Side::parse(string(value, "side")?)?,
            });
        }
        ensure!(
            anchors.len() == ports.len() && anchors.iter().all(|a| ports.contains_key(&a.pin)),
            "实物模板端子与锚点不一致 {identity}"
        );
        return Ok(Template {
            public,
            asset: Asset::Photograph(photo),
            size,
            anchors,
        });
    }

    let mut root = Element::parse(file(symbol)?)?;
    if variant {
        let layout = read(folder.join("channel-layout.json"))?;
        root = configure(root, &mut ports, &layout, &counts)?;
        public["model"] = json!(format_counts(string(&layout, "model")?, &counts));
    }
    let size = symbol_size(&root)?;
    let mut leads = HashSet::new();
    walk(&mut root, &mut |element| {
        if element.name == "line" {
            for (x, y) in [("x1", "y1"), ("x2", "y2")] {
                if let (Some(x), Some(y)) = (element.attributes.get(x), element.attributes.get(y)) {
                    leads.insert(point(x.parse()?, y.parse()?));
                }
            }
        }
        Ok(())
    })?;
    let mut anchors = Vec::new();
    let mut points = HashSet::new();
    let mut seen = HashSet::new();
    walk(&mut root, &mut |element| {
        if let Some(pin) = element.attributes.get("data-port") {
            let (x, y) = (coordinate(element, "cx")?, coordinate(element, "cy")?);
            let side = Side::parse(attribute(element, "data-side")?)?;
            ensure!(
                seen.insert(pin.clone())
                    && ports.contains_key(pin)
                    && (0.0..=size.0).contains(&x)
                    && (0.0..=size.1).contains(&y),
                "模板锚点无效 {identity}.{pin}"
            );
            ensure!(
                leads.contains(&point(x, y)),
                "模板引脚线与锚点有断口 {identity}.{pin}"
            );
            points.insert(point(x, y));
            anchors.push(Anchor {
                pin: pin.clone(),
                x,
                y,
                side,
            });
            element.attributes.insert("stroke".into(), color.into());
        }
        Ok(())
    })?;
    ensure!(
        variant || anchors.len() == ports.len(),
        "模板端子与锚点不一致 {identity}"
    );
    if variant {
        public["ports"] = json!(
            anchors
                .iter()
                .map(|anchor| &ports[&anchor.pin])
                .collect::<Vec<_>>()
        );
    }
    walk(&mut root, &mut |element| {
        if element.name == "line" {
            for (x, y) in [("x1", "y1"), ("x2", "y2")] {
                if let (Some(x), Some(y)) = (element.attributes.get(x), element.attributes.get(y)) {
                    if points.contains(&point(x.parse()?, y.parse()?)) {
                        element.attributes.insert("stroke".into(), color.into());
                        break;
                    }
                }
            }
        }
        if let Some(role) = element.attributes.get("data-circuit-role") {
            let key = match role.as_str() {
                "wire" => "stroke",
                "junction" => "fill",
                _ => bail!("{identity} 的电路图形角色无效 {role}"),
            };
            element.attributes.insert(key.into(), color.into());
        }
        if element.name == "text" {
            let label = element.get_text().unwrap_or_default();
            let role = element
                .attributes
                .get("data-text-role")
                .map(String::as_str)
                .or_else(|| ports.contains_key(label.trim()).then_some("terminal"));
            if let Some(role) = role {
                let fill = match role {
                    "terminal" | "interface" => color,
                    "reference" => "#172b3c",
                    "description" => "#465968",
                    "emergency" => "#a1273b",
                    _ => bail!("{identity} 的文字角色无效 {role}"),
                };
                element.attributes.insert("fill".into(), fill.into());
            }
        }
        Ok(())
    })?;
    root.attributes.insert("width".into(), size.0.to_string());
    root.attributes.insert("height".into(), size.1.to_string());
    let mut buffer = Vec::new();
    root.write_with_config(
        &mut buffer,
        xmltree::EmitterConfig::new().write_document_declaration(false),
    )?;
    Ok(Template {
        public,
        asset: Asset::Symbol(String::from_utf8(buffer)?),
        size,
        anchors,
    })
}

fn point(x: f32, y: f32) -> (u32, u32) {
    (
        if x == 0.0 { 0 } else { x.to_bits() },
        if y == 0.0 { 0 } else { y.to_bits() },
    )
}

fn attribute<'a>(element: &'a Element, name: &str) -> Result<&'a str> {
    element
        .attributes
        .get(name)
        .map(String::as_str)
        .with_context(|| format!("{} 缺少 {name}", element.name))
}

fn coordinate(element: &Element, name: &str) -> Result<f32> {
    Ok(attribute(element, name)?.parse()?)
}

fn symbol_size(root: &Element) -> Result<(f32, f32)> {
    let parts = attribute(root, "viewBox")?
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()?;
    let [x, y, w, h] = parts[..] else {
        bail!("模板 viewBox 应包含四个数值")
    };
    ensure!(
        x == 0.0 && y == 0.0 && w > 0.0 && h > 0.0 && w <= 10_000.0 && h <= 10_000.0,
        "模板尺寸无效"
    );
    Ok((w, h))
}

fn walk(element: &mut Element, visit: &mut impl FnMut(&mut Element) -> Result<()>) -> Result<()> {
    visit(element)?;
    for node in &mut element.children {
        if let XMLNode::Element(child) = node {
            walk(child, visit)?;
        }
    }
    Ok(())
}

fn move_y(element: &mut Element, offset: f32) -> Result<()> {
    for name in ["y", "y1", "y2", "cy"] {
        if let Some(value) = element.attributes.get_mut(name) {
            *value = (value.parse::<f32>()? + offset).to_string();
        }
    }
    Ok(())
}

fn set_text(element: &mut Element, label: String) {
    element.children = vec![XMLNode::Text(label)];
}

fn format_counts(pattern: &str, counts: &HashMap<String, i64>) -> String {
    counts
        .iter()
        .fold(pattern.to_owned(), |text, (key, value)| {
            text.replace(&format!("{{{key}}}"), &value.to_string())
        })
}

fn count(counts: &HashMap<String, i64>, name: &str) -> Result<i64> {
    counts
        .get(name)
        .copied()
        .with_context(|| format!("模板缺少参数 {name}"))
}

fn configure(
    mut root: Element,
    ports: &mut HashMap<String, Value>,
    layout: &Value,
    counts: &HashMap<String, i64>,
) -> Result<Element> {
    if layout["repeat_switches"] == true {
        let (w, h) = symbol_size(&root)?;
        let count = count(counts, "count")?;
        let start = self::count(counts, "start")?;
        let prototype = std::mem::take(&mut root.children);
        let existing = std::mem::take(ports);
        root.attributes
            .insert("viewBox".into(), format!("0 0 {w} {}", h * count as f32));
        for index in 0..count {
            let label = format!("S{}", start + index);
            let mut children = prototype.clone();
            for child in &mut children {
                if let XMLNode::Element(element) = child {
                    walk(element, &mut |element| {
                        move_y(element, index as f32 * h)?;
                        if let Some(pin) = element.attributes.get_mut("data-port") {
                            let (_, contact) =
                                pin.split_once('-').context("行程开关端子缺少触点编号")?;
                            let mut port = existing.get(pin).context("行程开关端子未定义")?.clone();
                            *pin = format!("{label}-{contact}");
                            port["id"] = json!(pin);
                            ports.insert(pin.clone(), port);
                        }
                        if element.attributes.contains_key("data-switch-label") {
                            set_text(element, label.clone());
                        }
                        Ok(())
                    })?;
                }
            }
            root.children.extend(children);
        }
    } else if let Some(pitch) = layout.get("relay_pitch") {
        repeat_relay(&mut root, ports, count(counts, "channels")?, number(pitch)?)?;
    } else if let Some(formats) = layout["labels"].as_object() {
        let mut labels: HashMap<_, _> = formats
            .iter()
            .map(|(name, pattern)| {
                Ok((
                    name.clone(),
                    format_counts(pattern.as_str().context("参数文字应为文本")?, counts),
                ))
            })
            .collect::<Result<_>>()?;
        ensure!(
            labels.values().collect::<HashSet<_>>().len() == labels.len(),
            "模块内部的开关位号不能重复"
        );
        walk(&mut root, &mut |element| {
            if let Some(name) = element.attributes.get("data-parameter-label") {
                let label = labels
                    .remove(name)
                    .context("模块图形参数文字未定义或重复")?;
                set_text(element, label);
            }
            Ok(())
        })?;
        ensure!(labels.is_empty(), "模块图形缺少参数文字位置");
    } else {
        root = channel_symbol(layout, counts)?;
    }
    Ok(root)
}

fn repeat_relay(
    root: &mut Element,
    ports: &mut HashMap<String, Value>,
    channels: i64,
    pitch: f32,
) -> Result<()> {
    let (w, h) = symbol_size(root)?;
    let row_key = "data-relay-channel";
    let rows: Vec<_> = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| {
            child
                .as_element()
                .filter(|row| row.attributes.contains_key(row_key))
                .map(|row| (index, row))
        })
        .collect();
    let Some(&(row_position, first)) = rows.first() else {
        bail!("继电器缺少第 1 组触点")
    };
    ensure!(
        attribute(first, row_key)? == "1",
        "继电器符号须包含第 1 组触点"
    );
    let prototype = first.clone();
    let delta = (channels as f32 - rows.len() as f32) * pitch;
    root.attributes
        .insert("viewBox".into(), format!("0 0 {w} {}", h + delta));
    let outline = root.get_mut_child("rect").context("继电器符号缺少外框")?;
    outline.attributes.insert(
        "height".into(),
        (coordinate(outline, "height")? + delta).to_string(),
    );
    let mut coil = false;
    let mut linkage = false;
    for child in root.children.iter_mut().filter_map(XMLNode::as_mut_element) {
        if child.attributes.contains_key("data-relay-coil") {
            coil = true;
            walk(child, &mut |element| move_y(element, delta))?;
        }
        if child.attributes.contains_key("data-relay-linkage") {
            linkage = true;
            let top = coordinate(child, "data-top")? as i64;
            let bottom = (coordinate(child, "data-bottom")? + delta) as i64;
            let x = attribute(child, "data-x")?;
            let path: Vec<_> = (top..bottom)
                .step_by(6)
                .map(|y| format!("M {x} {y} V {}", (y + 3).min(bottom)))
                .collect();
            child.attributes.insert("d".into(), path.join(" "));
        }
    }
    ensure!(coil && linkage, "继电器符号缺少线圈或联动线");
    root.children.retain(|child| {
        !child
            .as_element()
            .is_some_and(|row| row.attributes.contains_key(row_key))
    });
    for channel in 1..=channels {
        let mut row = prototype.clone();
        row.attributes.insert(row_key.into(), channel.to_string());
        let labels: HashMap<_, _> = ["COM", "NO", "NC"]
            .into_iter()
            .map(|kind| (format!("{kind}1"), (kind, format!("{kind}{channel}"))))
            .collect();
        walk(&mut row, &mut |element| {
            move_y(element, (channel - 1) as f32 * pitch)?;
            if let Some(pin) = element.attributes.get_mut("data-port") {
                let (kind, replacement) = labels.get(pin).context("继电器端子未定义")?;
                let mut data = ports.get(pin).context("继电器端子未定义")?.clone();
                *pin = replacement.clone();
                data["id"] = json!(pin);
                data["name"] = json!(format!("第 {channel} 组 {kind}"));
                ports.insert(pin.clone(), data);
            }
            if let Some((_, label)) = element
                .get_text()
                .and_then(|text| labels.get(text.as_ref()))
            {
                set_text(element, label.clone());
            }
            Ok(())
        })?;
        root.children
            .insert(row_position + (channel - 1) as usize, XMLNode::Element(row));
    }
    Ok(())
}

fn channel_symbol(layout: &Value, counts: &HashMap<String, i64>) -> Result<Element> {
    use std::fmt::Write;
    let w = number(&layout["width"])?;
    let pitch = number(&layout["pitch"])?;
    let first_y = number(&layout["first_y"])?;
    let gap = 7.0;
    let mut pins = Vec::new();
    let mut dividers = Vec::new();
    for (side, sections) in layout["sections"]
        .as_object()
        .context("参数化模块缺少端子分组")?
    {
        let left = side == "left";
        let mut y = first_y;
        for section in sections.as_array().context("端子分组应为数组")? {
            let labels = array(section, "pins")?
                .iter()
                .map(|pin| pin.as_str().context("端子应为文本"))
                .collect::<Result<Vec<_>>>()?;
            let groups = if let Some(parameter) = section["parameter"].as_str() {
                let count = count(counts, parameter)?;
                let half = (count + 1) / 2;
                let numbers: Vec<_> = match section["half"].as_str() {
                    Some("upper") => ((half + 1)..=count).collect(),
                    Some("lower") => (1..=half).rev().collect(),
                    _ => array(section, "numbers")?
                        .iter()
                        .filter_map(Value::as_i64)
                        .filter(|n| *n <= count)
                        .collect(),
                };
                numbers
                    .iter()
                    .map(|n| {
                        labels
                            .iter()
                            .map(|label| label.replace("{n}", &n.to_string()))
                            .collect()
                    })
                    .collect::<Vec<Vec<String>>>()
            } else {
                vec![labels.iter().map(|label| (*label).to_owned()).collect()]
            };
            if groups.is_empty() {
                continue;
            }
            if y != first_y {
                dividers.push((left, y - (pitch - gap) / 2.0, true));
                y += gap;
            }
            for (index, group) in groups.into_iter().enumerate() {
                if index != 0 && section["separate_channels"] == true {
                    dividers.push((left, y - pitch / 2.0, false));
                }
                for pin in group {
                    pins.push((left, y, pin));
                    y += pitch;
                }
            }
            for pin in section["trailing"].as_array().into_iter().flatten() {
                pins.push((left, y, pin.as_str().context("端子应为文本")?.to_owned()));
                y += pitch;
            }
        }
    }
    let h = pins
        .iter()
        .map(|(_, y, _)| *y)
        .reduce(f32::max)
        .context("模块图形缺少端子")?
        + 6.0;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}"><rect x="17" y="1" width="{}" height="{}" rx="5" fill="#f2f7fa" stroke="#6a879b"/>"##,
        w - 34.0,
        h - 2.0
    );
    let mid = w / 2.0;
    write!(
        svg,
        r##"<line x1="{mid}" y1="5" x2="{mid}" y2="{}" stroke="#c7d7e1" stroke-width=".6"/>"##,
        h - 5.0
    )?;
    for (left, y, section) in dividers {
        let (x1, x2) = if left { (18.0, mid) } else { (mid, w - 18.0) };
        let (ink, width) = if section {
            ("#a9bdca", ".85")
        } else {
            ("#d4e1e9", ".55")
        };
        write!(
            svg,
            r#"<line x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{ink}" stroke-width="{width}"/>"#
        )?;
    }
    for (left, y, pin) in pins {
        ensure!(is_pin(&pin), "无效的参数化端子 {pin}");
        let (x, edge, tx, side, align) = if left {
            (2.0, 17.0, 24.0, "left", "start")
        } else {
            (w - 2.0, w - 17.0, w - 24.0, "right", "end")
        };
        write!(
            svg,
            r##"<line x1="{x}" y1="{y}" x2="{edge}" y2="{y}" stroke="#6a879b" stroke-width=".8"/><circle cx="{x}" cy="{y}" r="1.65" fill="#fff" stroke="#285b7d" stroke-width=".7" data-port="{pin}" data-side="{side}"/><text x="{tx}" y="{}" text-anchor="{align}" font-size="8.8">{pin}</text>"##,
            y + 2.65
        )?;
    }
    svg.push_str("</svg>");
    Ok(Element::parse(svg.as_bytes())?)
}
