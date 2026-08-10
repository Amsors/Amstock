use std::{fmt, path::PathBuf, process::Stdio};

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::{
    error::{AppError, Result},
    models::Element,
};

#[derive(Debug, Clone, Copy)]
pub enum PrintMode {
    Preview,
    Printer,
}

impl PrintMode {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Preview => "preview",
            Self::Printer => "printer",
        }
    }
}

impl fmt::Display for PrintMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_arg())
    }
}

impl std::str::FromStr for PrintMode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        match value {
            "preview" => Ok(Self::Preview),
            "printer" => Ok(Self::Printer),
            _ => Err(format!("未知打印模式 {value:?}，可选值为 preview、printer")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrintConfig {
    pub mode: PrintMode,
    pub python: PathBuf,
    pub script: PathBuf,
    pub output_dir: PathBuf,
    pub host: String,
    pub port: u16,
    pub timeout: f64,
    pub open_preview: bool,
    pub cut: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum LabelStyle {
    A1,
    A2,
    B1,
    B2,
}

impl LabelStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A1 => "A1",
            Self::A2 => "A2",
            Self::B1 => "B1",
            Self::B2 => "B2",
        }
    }

    pub fn includes_children(self) -> bool {
        matches!(self, Self::B1 | Self::B2)
    }
}

#[derive(Debug, Serialize)]
struct ChildLabel<'a> {
    identifier: String,
    name: &'a str,
}

#[derive(Debug, Serialize)]
struct LabelPayload<'a> {
    schema_version: u8,
    style: &'static str,
    kind: &'a str,
    identifier: String,
    name: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<ChildLabel<'a>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrintOutput {
    pub schema_version: u8,
    pub mode: String,
    pub style: String,
    pub identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

fn payload<'a>(
    element: &'a Element,
    style: LabelStyle,
    children: &'a [Element],
) -> LabelPayload<'a> {
    LabelPayload {
        schema_version: 1,
        style: style.as_str(),
        kind: &element.kind,
        identifier: element.code(),
        name: &element.name,
        children: children
            .iter()
            .map(|child| ChildLabel {
                identifier: child.code(),
                name: &child.name,
            })
            .collect(),
    }
}

pub fn sort_children(children: &mut [Element]) {
    children.sort_by(|left, right| {
        left.tag_a
            .cmp(&right.tag_a)
            .then(left.tag_b.cmp(&right.tag_b))
            .then(left.tag_c.cmp(&right.tag_c))
            .then(left.serial.cmp(&right.serial))
    });
}

pub async fn print(
    config: &PrintConfig,
    element: &Element,
    style: LabelStyle,
    children: &[Element],
) -> Result<PrintOutput> {
    let input = serde_json::to_vec(&payload(element, style, children))
        .map_err(|error| AppError::Printer(format!("无法生成标签请求：{error}")))?;
    let mut command = tokio::process::Command::new(&config.python);
    command
        .arg(&config.script)
        .arg("--mode")
        .arg(config.mode.as_arg())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match config.mode {
        PrintMode::Preview => {
            command.arg("--output-dir").arg(&config.output_dir);
            if !config.open_preview {
                command.arg("--no-open");
            }
        }
        PrintMode::Printer => {
            command
                .arg("--host")
                .arg(&config.host)
                .arg("--port")
                .arg(config.port.to_string())
                .arg("--timeout")
                .arg(config.timeout.to_string());
            if !config.cut {
                command.arg("--no-cut");
            }
        }
    }

    let mut child = command.spawn().map_err(|error| {
        AppError::Printer(format!(
            "无法启动 Python 标签模块（{}）：{error}",
            config.python.display()
        ))
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Printer("无法连接 Python 标签模块的 stdin".into()))?;
    stdin
        .write_all(&input)
        .await
        .map_err(|error| AppError::Printer(format!("无法传递标签数据：{error}")))?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .map_err(|error| AppError::Printer(format!("等待 Python 标签模块失败：{error}")))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AppError::Printer(if detail.is_empty() {
            format!("Python 标签模块退出：{}", output.status)
        } else {
            detail
        }));
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|error| AppError::Printer(format!("Python 标签模块返回了无效结果：{error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(serial: i64, tag_a: &str, tag_b: i64, tag_c: i64) -> Element {
        Element {
            serial,
            kind: "item".into(),
            tag_a: tag_a.into(),
            tag_b,
            tag_c,
            name: format!("元素 {serial}"),
            description: String::new(),
            quantity: 1.0,
            unit: String::new(),
            parent_serial: None,
            image_mime: None,
            created_at: String::new(),
            updated_at: String::new(),
            deleted_at: None,
        }
    }

    #[test]
    fn b_label_children_sort_by_full_identifier_segments() {
        let mut children = vec![
            element(3, "B", 0, 0),
            element(2, "A", 2, 0),
            element(4, "A", 1, 9),
            element(1, "A", 1, 2),
        ];

        sort_children(&mut children);

        let codes: Vec<_> = children.iter().map(Element::code).collect();
        assert_eq!(
            codes,
            [
                "A-01-02-000001",
                "A-01-09-000004",
                "A-02-00-000002",
                "B-00-00-000003"
            ]
        );
    }
}
