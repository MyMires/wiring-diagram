use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::library::{self, Photo, Template};

#[derive(Deserialize)]
struct Project {
    version: u32,
    title: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    colors: HashMap<String, String>,
    sheets: Vec<Sheet>,
    instances: Vec<Device>,
    nets: Vec<Net>,
}

#[derive(Deserialize)]
pub struct Sheet {
    pub id: String,
    pub title: String,
    pub photo: Option<SheetPhoto>,
}

#[derive(Deserialize)]
pub struct SheetPhoto {
    pub file: String,
    pub caption: String,
    pub marks: Vec<Value>,
}

#[derive(Deserialize)]
pub struct Device {
    pub id: String,
    pub reference: String,
    pub sheet: String,
    template: String,
    view: Option<String>,
    parameters: Option<Value>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub note: String,
    pub station: Option<String>,
}

#[derive(Deserialize)]
struct Net {
    name: String,
    members: Vec<String>,
    #[serde(default)]
    external: bool,
}

pub struct Plan {
    pub title: String,
    pub source: String,
    pub internal: String,
    pub external: String,
    pub sheets: Vec<Sheet>,
    pub devices: Vec<(Device, Template)>,
    pub members: HashMap<String, String>,
    pub endpoint_names: HashMap<String, String>,
    pub nets: Map<String, Value>,
    root: std::path::PathBuf,
}

impl Plan {
    pub fn photo(&self, filename: &str) -> Result<Photo> {
        let path = self
            .root
            .join(filename)
            .canonicalize()
            .with_context(|| format!("无法读取工程实物照片 {filename}"))?;
        ensure!(
            path.starts_with(&self.root) && path.is_file(),
            "资源应是工程内的文件 {filename}"
        );
        Photo::from_bytes(&fs::read(path)?)
    }
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}

fn printable(value: &str) -> bool {
    !value.is_empty() && value == value.trim() && value.chars().all(|char| !char.is_control())
}

pub fn load(path: &Path) -> Result<Plan> {
    let project: Project = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("无法读取工程 {}", path.display()))?,
    )
    .with_context(|| format!("工程 JSON 无效 {}", path.display()))?;
    ensure!(project.version == 1, "工程版本应为 1");
    ensure!(
        !project.sheets.is_empty() && !project.instances.is_empty(),
        "工程至少需要一页和一个配件实例"
    );
    for (role, color) in &project.colors {
        ensure!(
            matches!(role.as_str(), "internal" | "external"),
            "工程 colors 仅支持 internal 或 external"
        );
        ensure!(
            color.len() == 7
                && color.starts_with('#')
                && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit),
            "工程 colors.{role} 应使用 #RRGGBB 格式"
        );
    }
    let color = |role| {
        project
            .colors
            .get(role)
            .map(String::as_str)
            .unwrap_or("#1c5c94")
            .to_owned()
    };
    let internal = color("internal");
    let external = color("external");
    let mut sheets = HashSet::new();
    for sheet in &project.sheets {
        ensure!(
            token(&sheet.id) && sheets.insert(sheet.id.clone()),
            "图页 {:?} 标识无效或重复",
            sheet.id
        );
    }
    let mut devices = Vec::new();
    let mut identifiers = HashSet::new();
    let mut references = HashSet::new();
    let mut endpoint_names = HashMap::new();
    let mut legacy_endpoints = HashMap::new();
    for device in project.instances {
        ensure!(
            token(&device.id) && identifiers.insert(device.id.clone()),
            "配件实例 {:?} 标识无效或重复",
            device.id
        );
        ensure!(
            token(&device.reference) && references.insert(device.reference.clone()),
            "位号 {:?} 标识无效或重复",
            device.reference
        );
        ensure!(
            sheets.contains(&device.sheet),
            "{} 引用了不存在的图页",
            device.reference
        );
        let template = library::load(
            &device.template,
            device.view.as_deref(),
            device.parameters.as_ref(),
            &internal,
        )?;
        if let Some(station) = &device.station {
            ensure!(printable(station), "{} 的站号应为文本", device.reference);
            ensure!(
                template.public["fields"].get("station").is_some(),
                "{} 的模板不支持站号",
                device.reference
            );
        }
        for anchor in &template.anchors {
            let key = format!("{}.{}", device.id, anchor.pin);
            let name = format!("{}.{}", device.reference, anchor.pin);
            if template.public["id"] == "中盛16入16出继电器模块" {
                for (earlier, current) in [("V+", "DC+"), ("V-", "DC-")] {
                    if anchor.pin == current
                        && !template.anchors.iter().any(|anchor| anchor.pin == earlier)
                    {
                        let previous = format!("{}.{earlier}", device.id);
                        endpoint_names.insert(previous.clone(), name.clone());
                        legacy_endpoints.insert(previous, key.clone());
                    }
                }
            }
            endpoint_names.insert(key, name);
        }
        devices.push((device, template));
    }

    let mut nets = Map::new();
    let mut members = HashMap::new();
    for net in project.nets {
        ensure!(
            printable(&net.name) && !nets.contains_key(&net.name),
            "网络 {:?} 标识无效或重复",
            net.name
        );
        let minimum = if net.external { 1 } else { 2 };
        ensure!(
            net.members.len() >= minimum,
            "网络 {} 至少需要 {minimum} 个端子",
            net.name
        );
        let mut endpoints = Vec::new();
        for endpoint in &net.members {
            let canonical = legacy_endpoints.get(endpoint).unwrap_or(endpoint);
            let name = endpoint_names
                .get(canonical)
                .with_context(|| format!("端子不存在或已加入网络 {endpoint}"))?;
            ensure!(
                members
                    .insert(canonical.clone(), net.name.clone())
                    .is_none(),
                "端子不存在或已加入网络 {endpoint}"
            );
            endpoints.push(name);
        }
        nets.insert(net.name, json!(endpoints));
    }
    let root = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()?;
    Ok(Plan {
        title: project.title,
        source: project.source,
        internal,
        external,
        sheets: project.sheets,
        devices,
        members,
        endpoint_names,
        nets,
        root,
    })
}
