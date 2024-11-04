use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    #[serde(rename = "graph")]
    pub graph_settings: GraphSettings,
    pub expressions: Expressions,
}

fn r#true() -> bool {
    true
}

fn is_true(x: &bool) -> bool {
    *x
}

fn is_false(x: &bool) -> bool {
    !*x
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphSettings {
    pub viewport: Viewport,
    #[serde(default = "r#true")]
    #[serde(skip_serializing_if = "is_true")]
    pub show_grid: bool,
    #[serde(default = "r#true")]
    #[serde(skip_serializing_if = "is_true")]
    pub show_x_axis: bool,
    #[serde(default = "r#true")]
    #[serde(skip_serializing_if = "is_true")]
    pub show_y_axis: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub degree_mode: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Viewport {
    #[serde(rename = "xmin")]
    pub x_min: f64,
    #[serde(rename = "ymin")]
    pub y_min: f64,
    #[serde(rename = "xmax")]
    pub x_max: f64,
    #[serde(rename = "ymax")]
    pub y_max: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Expressions {
    pub list: Vec<Cell>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticker: Option<Ticker>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Cell {
    Expression {
        id: String,
        // ExprItem is too large, so put it behind a Box.
        #[serde(flatten)]
        item: Box<ExprItem>,
    },
    Folder {
        id: String,
        title: String,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_false")]
        collapsed: bool,
        #[serde(default)]
        #[serde(skip_serializing_if = "is_false")]
        secret: bool,
    },
    #[serde(rename_all = "camelCase")]
    Text {
        id: String,
        folder_id: Option<String>,
        text: String,
    },
    Image,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExprItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latex: Option<String>,
    #[serde(flatten)]
    pub label: Label,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parametric_domain: Option<ParametricDomain>,
    #[serde(flatten)]
    pub line_options: LineOptions,
    #[serde(flatten)]
    pub point_options: PointOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_latex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clickable_info: Option<ClickableInfo>,
    #[serde(flatten)]
    pub fill_options: FillOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slider: Option<Slider>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slider {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub hard_min: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub hard_max: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParametricDomain {
    pub min: String,
    pub max: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub show_label: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_opacity: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_opacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_width: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PointOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub points: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_opacity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drag_mode: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticker {
    pub handler_latex: String,
    #[serde(default)]
    pub open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_step_latex: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClickableInfo {
    pub latex: String,
    #[serde(default)]
    pub enabled: bool,
}
