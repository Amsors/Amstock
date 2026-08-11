use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Element {
    pub serial: i64,
    pub kind: String,
    pub tag_a: String,
    pub tag_b: i64,
    pub tag_c: i64,
    pub name: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub parent_serial: Option<i64>,
    pub image_mime: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

impl Element {
    pub fn code(&self) -> String {
        format!(
            "{}-{:02}-{:02}-{:06}",
            self.tag_a, self.tag_b, self.tag_c, self.serial
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ElementView {
    #[serde(flatten)]
    pub element: Element,
    pub code: String,
    pub has_image: bool,
}

impl From<Element> for ElementView {
    fn from(element: Element) -> Self {
        let code = element.code();
        let has_image = element.image_mime.is_some();
        Self {
            element,
            code,
            has_image,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateElement {
    pub kind: String,
    pub tag_a: String,
    pub tag_b: i64,
    pub tag_c: i64,
    pub name: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub parent_serial: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateElement {
    pub kind: String,
    pub tag_a: String,
    pub tag_b: i64,
    pub tag_c: i64,
    pub name: String,
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub parent_serial: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryMapping {
    pub tag_a: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MnemonicMapping {
    pub tag_a: String,
    pub tag_b: i64,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MappingName {
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TreeNode {
    #[serde(flatten)]
    pub element: ElementView,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteMode {
    MoveToParent,
    MoveToContainer,
    Cascade,
}

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub mode: Option<DeleteMode>,
    pub target_serial: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub include_deleted: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ElementLookupView {
    pub element: ElementView,
    /// 从最外层父容器到当前元素，便于直接渲染面包屑路径。
    pub path: Vec<ElementView>,
}

#[derive(Debug, Deserialize)]
pub struct PrintRequest {
    pub style: crate::printing::LabelStyle,
}
