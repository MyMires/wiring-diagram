use std::{
    collections::HashMap,
    fmt::Write as _,
    io::Write as _,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{Context, Result, bail, ensure};
use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref, TextStr};
use svg2pdf::{ConversionOptions, usvg};

use crate::library::{Anchor, Asset, Photo, SIDES, Side, Template, photo_point, string};
use crate::project::{Device, Plan, SheetPhoto};

const FONT_BYTES: &[u8] = include_bytes!("../assets/NotoSansCJKsc-Regular.otf");
const FONT_NAME: &str = "Noto Sans CJK SC";
const PAGE: (f32, f32) = (842.0, 595.0);
const INK: &str = "#17293b";
const MUTED: &str = "#4f6378";
const RULE: &str = "#bfcdd6";
const ACCENT: &str = "#1c5c94";

type Point = (f32, f32);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy)]
struct Box2 {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Box2 {
    fn right(self) -> f32 {
        self.x + self.w
    }
    fn bottom(self) -> f32 {
        self.y + self.h
    }
    fn contains(self, other: Self) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.right() >= other.right()
            && self.bottom() >= other.bottom()
    }
    fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

fn font() -> &'static ttf_parser::Face<'static> {
    static FONT: OnceLock<ttf_parser::Face<'static>> = OnceLock::new();
    FONT.get_or_init(|| ttf_parser::Face::parse(FONT_BYTES, 0).expect("embedded Noto Sans font"))
}

fn width(text: &str, size: f32) -> f32 {
    let face = font();
    let advance: u32 = text
        .chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|glyph| face.glyph_hor_advance(glyph))
                .unwrap_or(face.units_per_em()) as u32
        })
        .sum();
    advance as f32 / face.units_per_em() as f32 * size
}

fn center_to_baseline(text: &str, size: f32) -> f32 {
    let face = font();
    let bounds = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .filter_map(|c| {
            face.glyph_index(c)
                .and_then(|id| face.glyph_bounding_box(id))
        })
        .fold(None, |extents, bounds| {
            let (low, high) = extents.unwrap_or((bounds.y_min, bounds.y_max));
            Some((low.min(bounds.y_min), high.max(bounds.y_max)))
        });
    let (low, high) = bounds.unwrap_or((face.descender(), face.ascender()));
    (f32::from(low) + f32::from(high)) * size / (2.0 * f32::from(face.units_per_em()))
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Default)]
struct Canvas(String);

impl Canvas {
    fn text(&mut self, (x, y): Point, value: &str, size: f32, color: &str, align: Align) {
        let anchor = match align {
            Align::Start => "start",
            Align::Middle => "middle",
            Align::End => "end",
        };
        let value = escaped(value);
        write!(self.0, r#"<text x="{x}" y="{y}" font-family="{FONT_NAME}" font-size="{size}" fill="{color}" text-anchor="{anchor}">{value}</text>"#).unwrap();
    }

    fn line(&mut self, (x1, y1): Point, (x2, y2): Point, color: &str, width: f32) {
        write!(self.0, r#"<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{color}" stroke-width="{width}"/>"#).unwrap();
    }

    fn wire(&mut self, (x, y): Point, (ex, ey): Point, (tx, ty): Point, color: &str) {
        write!(self.0, r#"<polyline points="{x},{y} {ex},{ey} {tx},{ty}" fill="none" stroke="{color}" stroke-width="1.05"/>"#).unwrap();
    }

    fn rect(&mut self, Box2 { x, y, w, h }: Box2, stroke: &str, fill: &str, thickness: f32) {
        write!(self.0, r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" stroke="{stroke}" fill="{fill}" stroke-width="{thickness}"/>"#).unwrap();
    }

    fn circle(&mut self, (x, y): Point, radius: f32, color: &str, thickness: f32) {
        write!(self.0, r#"<circle cx="{x}" cy="{y}" r="{radius}" stroke="{color}" fill="none" stroke-width="{thickness}"/>"#).unwrap();
    }

    fn photo(&mut self, Box2 { x, y, w, h }: Box2, photo: &Photo) {
        write!(
            self.0,
            r#"<image x="{x}" y="{y}" width="{w}" height="{h}" xlink:href="{}"/>"#,
            photo.url
        )
        .unwrap();
    }
}

struct Block<'a> {
    template: &'a Template,
    heading: String,
    note: String,
    groups: [Vec<(&'a Anchor, &'a str)>; 4],
    insets: (f32, f32, f32, f32),
    font_size: f32,
    spacing: f32,
}

impl<'a> Block<'a> {
    fn new(device: &Device, template: &'a Template, plan: &'a Plan) -> Result<Self> {
        let heading = format!(
            "{}   {}",
            device.reference,
            if device.name.is_empty() {
                string(&template.public, "name")?
            } else {
                &device.name
            }
        );
        let mut note = if device.note.is_empty() {
            string(&template.public, "model")?
        } else {
            &device.note
        }
        .to_owned();
        if let Some(station) = &device.station {
            write!(note, " · 站号 {station}")?;
        }
        let groups = SIDES.map(|side| {
            template
                .anchors
                .iter()
                .filter(|anchor| anchor.side == side)
                .filter_map(|anchor| {
                    let key = format!("{}.{}", device.id, anchor.pin);
                    plan.members.get(&key).map(|name| (anchor, name.as_str()))
                })
                .collect::<Vec<_>>()
        });
        let compact = template.public["parameters"]
            .as_object()
            .is_some_and(|p| !p.is_empty())
            || groups.iter().any(|group| group.len() >= 16);
        let (font_size, spacing) = if compact { (8.0, 9.0) } else { (10.0, 15.0) };
        let padding = if matches!(template.asset, Asset::Photograph(_)) {
            12.0
        } else {
            6.0
        };
        let reserve = groups.each_ref().map(|group| {
            group
                .iter()
                .map(|(_, name)| width(name, font_size) + 32.0)
                .reduce(f32::max)
                .unwrap_or(padding)
        });
        let insets = (
            reserve[0],
            38.0 + padding + groups[2].len() as f32 * spacing,
            reserve[1].max(reserve[2]).max(reserve[3]),
            padding + groups[3].len() as f32 * spacing,
        );
        Ok(Self {
            template,
            heading,
            note,
            groups,
            insets,
            font_size,
            spacing,
        })
    }

    fn horizontal_space(&self, scale: f32) -> (f32, f32) {
        let half_symbol = self.template.size.0 * scale / 2.0;
        let half_caption = width(&self.heading, 14.0).max(width(&self.note, 9.0)) / 2.0 + 10.0;
        (
            (half_symbol + self.insets.0).max(half_caption),
            (half_symbol + self.insets.2).max(half_caption),
        )
    }

    fn size(&self, scale: f32) -> (f32, f32) {
        let (left, right) = self.horizontal_space(scale);
        let rows = self.groups[0].len().max(self.groups[1].len());
        let min_height = 60.0 + rows.saturating_sub(1) as f32 * self.spacing;
        (
            left + right,
            min_height.max(self.template.size.1 * scale + self.insets.1 + self.insets.3),
        )
    }

    fn scale(&self, factor: f32) -> Result<f32> {
        if matches!(self.template.asset, Asset::Photograph(_)) {
            return Ok((0.75 * factor).max(0.55));
        }
        let min = self.template.public["min_scale"]
            .as_f64()
            .context("模板缺少 min_scale")? as f32;
        Ok((0.95 * factor).max(min))
    }
}

fn place_rows(preferred: Vec<f32>, low: f32, high: f32, spacing: f32) -> Result<Vec<f32>> {
    let count = preferred.len();
    ensure!(
        count <= 1 || high - low >= (count - 1) as f32 * spacing,
        "网络标签空间不足，请分图页"
    );
    let mut rows = Vec::with_capacity(count);
    for (index, position) in preferred.into_iter().enumerate() {
        let floor = rows.last().map(|last| last + spacing).unwrap_or(low);
        let ceiling = high - (count - index - 1) as f32 * spacing;
        rows.push(floor.max(position).min(ceiling));
    }
    Ok(rows)
}

fn pack(sizes: &[(f32, f32)], (available_width, available_height): Point) -> Option<Vec<Point>> {
    let (mut x, mut y, mut row_height) = (0.0, 0.0, 0.0_f32);
    let mut positions = Vec::with_capacity(sizes.len());
    for &(w, h) in sizes {
        if x != 0.0 && x + w > available_width {
            (x, y, row_height) = (0.0, y + row_height + 12.0, 0.0);
        }
        if w > available_width || y + h > available_height {
            return None;
        }
        positions.push((x, y));
        x += w + 12.0;
        row_height = row_height.max(h);
    }
    Some(positions)
}

fn layout(
    blocks: &[Block<'_>],
    (page_width, page_height): Point,
    sheet: &str,
) -> Result<Vec<(Point, f32)>> {
    for factor in [1.0, 0.9, 0.8, 0.7, 0.0] {
        let scales = blocks
            .iter()
            .map(|block| block.scale(factor))
            .collect::<Result<Vec<_>>>()?;
        let sizes: Vec<_> = blocks
            .iter()
            .zip(&scales)
            .map(|(block, scale)| block.size(*scale))
            .collect();
        if let Some(positions) = pack(&sizes, (page_width, page_height)) {
            return Ok(positions.into_iter().zip(scales).collect());
        }
        let columns: Vec<_> = sizes.iter().map(|&(w, h)| (h, w)).collect();
        if let Some(positions) = pack(&columns, (page_height, page_width)) {
            return Ok(positions
                .into_iter()
                .map(|(y, x)| (x, y))
                .zip(scales)
                .collect());
        }
    }
    bail!("图页 {sheet} 的模块及网络标签空间不足，请将部分模块移至其他图页")
}

fn draw_device(
    page: &mut Canvas,
    (x, y): Point,
    block: &Block<'_>,
    scale: f32,
    plan: &Plan,
) -> Result<()> {
    let (dw, dh) = block.size(scale);
    let center = x + block.horizontal_space(scale).0;
    page.text((center, y + 19.0), &block.heading, 14.0, INK, Align::Middle);
    page.text((center, y + 32.0), &block.note, 9.0, MUTED, Align::Middle);
    page.line((x + 10.0, y + 38.0), (x + dw - 10.0, y + 38.0), RULE, 0.5);
    let (w, h) = block.template.size;
    let frame = Box2 {
        x: center - w * scale / 2.0,
        y: y + block.insets.1,
        w: w * scale,
        h: h * scale,
    };
    match &block.template.asset {
        Asset::Symbol(svg) => write!(
            page.0,
            r#"<g transform="translate({} {}) scale({scale})" font-family="{FONT_NAME}">{svg}</g>"#,
            frame.x, frame.y
        )?,
        Asset::Photograph(photo) => page.photo(frame, photo),
    }
    let mut marks = Vec::new();
    for (side, entries) in SIDES.into_iter().zip(&block.groups) {
        let vertical = matches!(side, Side::Top | Side::Bottom);
        let mut entries = entries.clone();
        entries.sort_by(|(a, _), (b, _)| {
            if vertical {
                a.x.total_cmp(&b.x)
            } else {
                a.y.total_cmp(&b.y)
            }
        });
        let rows = if vertical {
            Vec::new()
        } else {
            place_rows(
                entries.iter().map(|(a, _)| frame.y + a.y * scale).collect(),
                y + 50.0,
                y + dh - 10.0,
                block.spacing,
            )?
        };
        let count = entries.len();
        for (index, (anchor, name)) in entries.into_iter().enumerate() {
            let start = (frame.x + anchor.x * scale, frame.y + anchor.y * scale);
            let left = side == Side::Left;
            let (row, elbow) = if vertical {
                let offset = 10.0 + (count - index - 1) as f32 * block.spacing;
                let row = if side == Side::Top {
                    frame.y - offset
                } else {
                    frame.bottom() + offset
                };
                (row, (start.0, row))
            } else {
                (
                    rows[index],
                    (start.0 + if left { -6.0 } else { 6.0 }, start.1),
                )
            };
            let rail = if left {
                frame.x - 20.0
            } else {
                frame.right() + 20.0
            };
            page.wire(start, elbow, (rail, row), &plan.external);
            let (tx, align) = if left {
                (rail - 5.0, Align::End)
            } else {
                (rail + 5.0, Align::Start)
            };
            page.text(
                (tx, row + center_to_baseline(name, block.font_size)),
                name,
                block.font_size,
                &plan.external,
                align,
            );
            marks.push((anchor.pin.as_str(), start));
        }
    }
    if matches!(block.template.asset, Asset::Photograph(_)) {
        draw_marks(page, frame, &marks, &plan.internal, true)?;
    }
    Ok(())
}

fn draw_marks(
    page: &mut Canvas,
    frame: Box2,
    marks: &[(&str, Point)],
    color: &str,
    compact: bool,
) -> Result<()> {
    let (size, height, radius, inset, padding, half) = if compact {
        (8.0, 12.0, 3.0, 16.0, 4.0, 6.0)
    } else {
        (10.0, 17.0, 5.0, 31.0, 6.0, 8.0)
    };
    let mut boxes = Vec::new();
    for direction in [1.0, -1.0] {
        let mut entries: Vec<_> = marks
            .iter()
            .filter_map(|&(name, (x, y))| {
                let w = width(name, size) + 2.0 * padding;
                let right = x + inset + w <= frame.right();
                (right == (direction > 0.0)).then_some((name, x, y, w))
            })
            .collect();
        entries.sort_by(|a, b| a.2.total_cmp(&b.2));
        let low = frame.y + 1.0;
        let high = frame.bottom() - height - 1.0;
        let preferred: Vec<_> = entries.iter().map(|entry| entry.2 - half).collect();
        let rows = if compact {
            place_rows(preferred, low, high, height + 1.0)?
        } else {
            preferred
                .into_iter()
                .map(|y| y.min(high).max(low))
                .collect()
        };
        for ((name, x, y, w), ty) in entries.into_iter().zip(rows) {
            let tx = if direction > 0.0 {
                x + inset
            } else {
                x - inset - w
            };
            let tag = Box2 {
                x: tx,
                y: ty,
                w,
                h: height,
            };
            ensure!(
                frame.contains(tag) && !boxes.iter().any(|&other| tag.intersects(other)),
                "照片标签空间不足 {name}"
            );
            boxes.push(tag);
            page.circle((x, y), radius, color, if compact { 1.15 } else { 1.6 });
            page.line(
                (x + radius * direction, y),
                (if direction > 0.0 { tx } else { tx + w }, ty + half),
                color,
                if compact { 0.75 } else { 1.0 },
            );
            page.rect(tag, color, "#ffffff", 0.65);
            page.text(
                (tx + padding, ty + half + center_to_baseline(name, size)),
                name,
                size,
                color,
                Align::Start,
            );
        }
    }
    Ok(())
}

fn draw_sheet_photo(page: &mut Canvas, photo: &SheetPhoto, plan: &Plan) -> Result<()> {
    let frame = Box2 {
        x: 410.0,
        y: 108.0,
        w: 400.0,
        h: 306.0,
    };
    page.rect(frame, RULE, "#f7fafc", 0.65);
    page.text(
        (frame.x + 10.0, frame.y + 23.0),
        "实物接口",
        16.0,
        INK,
        Align::Start,
    );
    let image = plan.photo(&photo.file)?;
    let scale = ((frame.w - 20.0) / image.width).min((frame.h - 85.0) / image.height);
    let target = Box2 {
        x: frame.x + 10.0,
        y: frame.y + 50.0,
        w: image.width * scale,
        h: image.height * scale,
    };
    page.photo(target, &image);
    ensure!(width(&photo.caption, 9.0) < frame.w - 20.0, "照片说明过长");
    page.text(
        (target.x, target.bottom() + 18.0),
        &photo.caption,
        9.0,
        MUTED,
        Align::Start,
    );
    let marks = photo
        .marks
        .iter()
        .map(|mark| {
            let endpoint = string(mark, "endpoint")?;
            let name = plan
                .endpoint_names
                .get(endpoint)
                .with_context(|| format!("照片端子引用无效 {endpoint}"))?;
            let (x, y) = photo_point(mark)?;
            Ok((
                name.as_str(),
                (target.x + x * target.w, target.y + y * target.h),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    draw_marks(page, target, &marks, &plan.internal, false)
}

pub struct Page {
    title: String,
    project_title: String,
    source: String,
    svg: String,
}

pub fn compose(plan: &Plan) -> Result<Vec<Page>> {
    ensure!(
        width(&plan.title, 11.0) < 610.0 && width(&plan.source, 9.0) < 778.0,
        "工程标题或来源说明过长"
    );
    let count = plan.sheets.len();
    plan.sheets.iter().enumerate().map(|(number, sheet)| {
        ensure!(width(&sheet.title, 27.0) < 610.0, "图页 {} 的标题过长", sheet.id);
        let mut page = Canvas::default();
        page.rect(Box2 { x: 0.0, y: 0.0, w: PAGE.0, h: PAGE.1 }, "none", "#ffffff", 0.0);
        page.rect(Box2 { x: 32.0, y: 32.0, w: 4.0, h: 46.0 }, "none", ACCENT, 0.0);
        page.text((48.0, 41.0), &plan.title, 11.0, MUTED, Align::Start);
        page.text((48.0, 72.0), &sheet.title, 27.0, INK, Align::Start);
        page.text((808.0, 58.0), &format!("{} / {count:02}", sheet.id), 13.0, ACCENT, Align::End);
        page.line((32.0, 91.0), (810.0, 91.0), RULE, 0.65);
        page.text((32.0, 570.0), &plan.source, 9.0, MUTED, Align::Start);
        page.line((32.0, 550.0), (810.0, 550.0), RULE, 0.65);
        let blocks = plan.devices.iter().filter(|(device, _)| device.sheet == sheet.id)
            .map(|(device, template)| Block::new(device, template, plan)).collect::<Result<Vec<_>>>()?;
        ensure!(!blocks.is_empty(), "图页 {} 缺少配件", sheet.id);
        let edge = if sheet.photo.is_some() { 394.0 } else { 810.0 };
        for (block, ((x, y), scale)) in blocks.iter().zip(layout(&blocks, (edge - 32.0, 430.0), &sheet.id)?) {
            draw_device(&mut page, (32.0 + x, 108.0 + y), block, scale, plan)?;
        }
        if let Some(photo) = &sheet.photo { draw_sheet_photo(&mut page, photo, plan)?; }
        Ok(Page {
            title: format!("{} · 第 {} 页", plan.title, number + 1),
            project_title: plan.title.clone(),
            source: plan.source.clone(),
            svg: format!(r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{}" height="{}" viewBox="0 0 {} {}">{}</svg>"#, PAGE.0, PAGE.1, PAGE.0, PAGE.1, page.0),
        })
    }).collect()
}

fn tree(page: &Page) -> Result<usvg::Tree> {
    static FONTS: OnceLock<std::sync::Arc<usvg::fontdb::Database>> = OnceLock::new();
    let fontdb = FONTS.get_or_init(|| {
        let mut database = usvg::fontdb::Database::new();
        database.load_font_data(FONT_BYTES.to_vec());
        database.set_sans_serif_family(FONT_NAME);
        database.set_monospace_family(FONT_NAME);
        std::sync::Arc::new(database)
    });
    let options = usvg::Options {
        font_family: FONT_NAME.into(),
        fontdb: fontdb.clone(),
        ..Default::default()
    };
    usvg::Tree::from_str(&page.svg, &options).context("无法处理 SVG 图面")
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn stage(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new_in(parent(path))?;
    file.write_all(bytes)?;
    Ok(file)
}

fn collect_text(element: &xmltree::Element, output: &mut Vec<String>) {
    if element.name == "text" {
        if let Some(content) = element.get_text() {
            output.push(content.into_owned());
        }
    }
    for child in element
        .children
        .iter()
        .filter_map(xmltree::XMLNode::as_element)
    {
        collect_text(child, output);
    }
}

pub fn write_svg(pages: &[Page], output: &Path) -> Result<Vec<PathBuf>> {
    let stem = output
        .file_stem()
        .context("输出路径缺少文件名")?
        .to_string_lossy();
    let mut outputs = Vec::new();
    for (number, page) in pages.iter().enumerate() {
        let output = if pages.len() == 1 {
            output.to_path_buf()
        } else {
            output.with_file_name(format!(
                "{stem}-p{:02}.{}",
                number + 1,
                output.extension().unwrap().to_string_lossy()
            ))
        };
        let mut text = Vec::new();
        collect_text(&xmltree::Element::parse(page.svg.as_bytes())?, &mut text);
        let mut svg = tree(page)?.to_string(&usvg::WriteOptions::default());
        let start = svg.find('>').context("生成 SVG 缺少根元素")? + 1;
        svg.insert_str(
            start,
            &format!(
                "<title>{}</title><desc>{}</desc>",
                escaped(&page.title),
                escaped(&text.join("\n"))
            ),
        );
        let temporary = stage(&output, svg.as_bytes())?;
        outputs.push((temporary, output));
    }
    outputs
        .into_iter()
        .map(|(temp, path)| {
            temp.persist(&path)?;
            Ok(path)
        })
        .collect()
}

pub fn write_pdf(pages: &[Page], output: &Path) -> Result<()> {
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let page_tree = alloc.bump();
    let info = alloc.bump();
    let ids: Vec<_> = pages.iter().map(|_| alloc.bump()).collect();
    let mut pdf = Pdf::new();
    pdf.catalog(catalog).pages(page_tree);
    pdf.pages(page_tree)
        .kids(ids.iter().copied())
        .count(ids.len() as i32);
    if let Some(first) = pages.first() {
        pdf.document_info(info)
            .title(TextStr(&first.project_title))
            .subject(TextStr(&first.source));
    }
    let form_name = Name(b"Wiring");
    for (id, page) in ids.into_iter().zip(pages) {
        let svg = tree(page)?;
        let (chunk, form) = svg2pdf::to_chunk(&svg, ConversionOptions::default())
            .map_err(|error| anyhow::anyhow!("无法转换 SVG 为矢量 PDF：{error:?}"))?;
        let mut mapping = HashMap::new();
        let chunk = chunk.renumber(|old| *mapping.entry(old).or_insert_with(|| alloc.bump()));
        let form = mapping[&form];
        let content_id = alloc.bump();
        let mut content = Content::new();
        content
            .transform([PAGE.0, 0.0, 0.0, PAGE.1, 0.0, 0.0])
            .x_object(form_name);
        let mut writer = pdf.page(id);
        writer
            .parent(page_tree)
            .media_box(Rect::new(0.0, 0.0, PAGE.0, PAGE.1))
            .contents(content_id);
        writer.resources().x_objects().pair(form_name, form);
        writer.finish();
        pdf.stream(content_id, &content.finish());
        pdf.extend(&chunk);
    }
    stage(output, &pdf.finish())?.persist(output)?;
    Ok(())
}
