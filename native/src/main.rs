mod drawing;
mod library;
mod project;

use std::{io, path::Path};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

const HELP: &str = "测试治具接线图\n用法：\n  wiring --version\n  wiring --licenses\n  wiring part [型号[@版本]] [--parameters JSON]\n  wiring validate 工程.json\n  wiring render 工程.json 接线图.svg|接线图.pdf";

fn query(identity: &str, parameters: Option<&str>) -> Result<Value> {
    let parameters: Option<Value> = parameters
        .map(serde_json::from_str)
        .transpose()
        .context("模板参数应为 JSON")?;
    let mut public = library::load(identity, None, parameters.as_ref(), "#1c5c94")?.public;
    if let Some(fields) = public.as_object_mut() {
        fields.shift_remove("min_scale");
    }
    Ok(public)
}

fn run(args: &[String]) -> Result<Option<Value>> {
    let result = match args {
        [flag] if flag == "--version" => {
            println!("wiring {}", env!("CARGO_PKG_VERSION"));
            return Ok(None);
        }
        [flag] if flag == "--licenses" => {
            io::copy(
                &mut include_bytes!("../assets/THIRD-PARTY-NOTICES.txt").as_slice(),
                &mut io::stdout().lock(),
            )?;
            return Ok(None);
        }
        [flag] if flag == "--help" || flag == "-h" => {
            println!("{HELP}");
            return Ok(None);
        }
        [command, flag]
            if matches!(command.as_str(), "part" | "validate" | "render")
                && matches!(flag.as_str(), "--help" | "-h") =>
        {
            println!("{HELP}");
            return Ok(None);
        }
        [command, rest @ ..] if command == "part" => match rest {
            [] => json!(library::identities()),
            [identity] => query(identity, None)?,
            [identity, flag, value] if flag == "--parameters" => query(identity, Some(value))?,
            [flag, value, identity] if flag == "--parameters" => query(identity, Some(value))?,
            _ => bail!("part 参数无效\n{HELP}"),
        },
        [command, input] if command == "validate" => {
            let plan = project::load(Path::new(input))?;
            let pages = drawing::compose(&plan)?;
            json!({"pages": pages.len(), "nets": plan.nets})
        }
        [command, input, output] if command == "render" => {
            let input = Path::new(input);
            let output = Path::new(output);
            let extension = output
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            ensure!(
                matches!(extension.as_str(), "svg" | "pdf") && !same_path(input, output),
                "输出路径应是单独的 PDF 或 SVG 文件"
            );
            let plan = project::load(input)?;
            let pages = drawing::compose(&plan)?;
            let mut result = json!({"pages": pages.len(), "nets": plan.nets});
            match extension.as_str() {
                "svg" => result["svg"] = json!(drawing::write_svg(&pages, output)?),
                "pdf" => {
                    drawing::write_pdf(&pages, output)?;
                    result["pdf"] = json!(output);
                }
                _ => unreachable!(),
            }
            result
        }
        _ => bail!("命令或参数无效\n{HELP}"),
    };
    Ok(Some(result))
}

fn same_path(input: &Path, output: &Path) -> bool {
    input == output
        || input
            .canonicalize()
            .ok()
            .zip(output.canonicalize().ok())
            .is_some_and(|(a, b)| a == b)
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = run(&args).and_then(|result| {
        if let Some(value) = result {
            serde_json::to_writer_pretty(io::stdout().lock(), &value)?;
            println!();
        }
        Ok(())
    });
    if let Err(error) = result {
        eprintln!("接线图校验失败：{error:#}");
        std::process::exit(2);
    }
}
