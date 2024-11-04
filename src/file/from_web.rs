use crate::file::{
    CanDragAxis, File, FillSettings, Folder, LineSettings, PointSettings, Settings, ShowGraphParts,
    Statement, Ticker, VisualSettings,
};
use crate::utils::{iter, IdVec, OptExt, SparseVec};
use glam::DVec2;
use http_req::error::Error as HttpError;
use http_req::request;
use http_req::response::StatusCode;
use std::collections::HashMap;
use std::fs;
use std::io::{stdout, Write};
use std::ops::Deref;
use std::string::FromUtf8Error;
use std::sync::{LazyLock, Mutex};
use thiserror::Error;

mod json_tree;

#[derive(Error, Debug)]
pub enum DesmosFileError {
    #[error("Non-success status code")]
    Status(StatusCode),
    #[error("Unrecognized drag mode")]
    DragMode(String),
    #[error("Http error")]
    Http(#[from] HttpError),
    #[error("Http response not UTF-8 compatible")]
    StringConv(#[from] FromUtf8Error),
    #[error("Deserialization failed")]
    Serde(#[from] serde_json::Error),
}

impl From<json_tree::LineOptions> for LineSettings {
    fn from(value: json_tree::LineOptions) -> Self {
        Self {
            show: value.lines.unwrap_or(false),
            opacity: value.line_opacity,
            width: value.line_width,
        }
    }
}
impl From<json_tree::FillOptions> for FillSettings {
    fn from(value: json_tree::FillOptions) -> Self {
        Self {
            show: value.fill.unwrap_or(true),
            opacity: value.fill_opacity,
        }
    }
}

impl TryFrom<json_tree::PointOptions> for PointSettings {
    type Error = DesmosFileError;

    fn try_from(value: json_tree::PointOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            show: value.points.unwrap_or(true),
            radius: value.point_size,
            opacity: value.point_opacity,
            drag_axis: if let Some(drag_mode) = value.drag_mode {
                CanDragAxis::try_from(drag_mode)?
            } else {
                CanDragAxis::default()
            },
        })
    }
}

impl TryFrom<String> for CanDragAxis {
    type Error = DesmosFileError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "NONE" => CanDragAxis::empty(),
            "X" => CanDragAxis::X_AXIS,
            "Y" => CanDragAxis::Y_AXIS,
            "XY" => CanDragAxis::all(),

            _ => return Err(DesmosFileError::DragMode(value)),
        })
    }
}

impl TryFrom<json_tree::File> for File {
    type Error = DesmosFileError;

    fn try_from(value: json_tree::File) -> Result<Self, Self::Error> {
        let json_tree::File {
            graph_settings,
            expressions,
        } = value;

        let json_tree::Expressions { ticker, list } = expressions;

        let mut maybe_folders = SparseVec::<Folder>::new();
        let mut statements = Vec::new();

        let folder_ids: IdVec<String> = IdVec::new();

        for cell in list {
            match cell {
                json_tree::Cell::Image => {}
                json_tree::Cell::Expression { id: _, item } => {
                    let json_tree::ExprItem {
                        folder_id,
                        latex,
                        line_options,
                        point_options,
                        fill_options,
                        ..
                    } = *item;

                    let folder_id = folder_id.map(|folder_id| folder_ids.id_or_insert(folder_id));

                    statements.push(Statement {
                        folder_id,
                        visual_settings: VisualSettings {
                            line_settings: line_options.into(),
                            point_settings: point_options.try_into()?,
                            fill_settings: fill_options.into(),
                        },
                        expr: latex.unwrap_or_else(String::new),
                    });
                }
                json_tree::Cell::Text { .. } => {}
                json_tree::Cell::Folder {
                    id,
                    secret: _,
                    title,
                    collapsed,
                } => {
                    let id = folder_ids.id_or_insert(id);
                    maybe_folders.insert(id, Folder { title, collapsed });
                }
            }
        }

        let mut folders = Vec::with_capacity(maybe_folders.count_elements());
        let mut folder_map = HashMap::with_capacity(maybe_folders.len());

        for (id, maybe_folder) in maybe_folders.into_inner().into_iter().enumerate() {
            if let Some(folder) = maybe_folder {
                let new_id = folders.len();
                folders.push(folder);
                folder_map.insert(id, new_id);
            }
        }

        for statement in statements.iter_mut() {
            if let Some(id) = statement.folder_id {
                let new_id = folder_map.get(&id).copied();
                statement.folder_id = new_id;
            }
        }

        Ok(Self {
            settings: Settings::from(graph_settings),
            folders,
            statements,
            ticker: ticker.map(Ticker::from),
        })
    }
}

const REQ_CACHE_PATH: &str = "cache.json";
static REQ_CACHE: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(|| {
    let cache = fs::read_to_string(REQ_CACHE_PATH).ok();
    Mutex::new(
        cache
            .and_then(|cache| serde_json::from_str::<HashMap<String, String>>(&cache).ok())
            .unwrap_or_default(),
    )
});

impl File {
    pub fn try_from_direct_url(url: &str) -> Result<Self, DesmosFileError> {
        let [id, _] = iter::into_exactly(url.rsplitn(2, "/"));
        Self::try_from_id(id)
    }

    pub fn try_from_id(id: &str) -> Result<Self, DesmosFileError> {
        Self::try_from_json_url(&format!(
            "https://www.desmos.com/calc-states/production/{id}"
        ))
    }

    pub fn try_from_json_url(url: &str) -> Result<Self, DesmosFileError> {
        let mut cache = REQ_CACHE.lock().unwrap_unreach();
        if let Some(cached) = cache.get(url) {
            let file = serde_json::from_str::<json_tree::File>(cached)?;
            File::try_from(file)
        } else {
            let mut data = Vec::new();
            println!("Requesting to url: {url}");
            let response = request::get(url, &mut data)?;
            if !response.status_code().is_success() {
                return Err(DesmosFileError::Status(response.status_code()));
            }
            let string = String::from_utf8(data)?;
            cache.insert(url.to_string(), string.to_string());
            let serialized = serde_json::to_string(cache.deref()).unwrap_unreach();
            let _ = fs::write(REQ_CACHE_PATH, serialized);

            let file = serde_json::from_str::<json_tree::File>(&string)?;
            File::try_from(file)
        }
    }
}

impl From<json_tree::Ticker> for Ticker {
    fn from(value: json_tree::Ticker) -> Self {
        Self {
            expr: value.handler_latex,
            freq: value
                .min_step_latex
                .and_then(|latex| latex.parse::<f64>().ok())
                .unwrap_or(60.0),
        }
    }
}

impl From<json_tree::GraphSettings> for Settings {
    fn from(value: json_tree::GraphSettings) -> Self {
        let show_graph_parts = [
            value.show_x_axis.then_some(ShowGraphParts::X_AXIS),
            value.show_y_axis.then_some(ShowGraphParts::Y_AXIS),
            value.show_grid.then_some(ShowGraphParts::GRID),
        ]
        .into_iter()
        .flatten()
        .collect();

        let viewport = value.viewport;
        let cam_target = DVec2::new(
            viewport.x_max + viewport.x_min,
            viewport.y_max + viewport.y_min,
        ) / 2.0;

        let ranges = [
            viewport.x_max - viewport.x_min,
            viewport.y_max - viewport.y_min,
        ];
        let [zoom_x, zoom_y] = ranges.map(f64::recip);
        let zoom = f64::min(zoom_x, zoom_y);

        Self {
            show_graph_parts,
            degree_mode: value.degree_mode,
            cam_target,
            zoom,
        }
    }
}

fn test_case(path: &str) {
    print!("Testing {path}...");
    stdout().flush().unwrap();
    let json_path = format!("tests/{path}.json");
    let result_path = format!("tests/{path}_result.json");
    let file_result_path = format!("tests/{path}_file_result.txt");

    // write the serialized-deserialized result to disk for comparison
    let string = fs::read_to_string(json_path).unwrap();
    let result = serde_json::from_str::<json_tree::File>(&string);
    let result = result.unwrap();
    fs::write(&result_path, serde_json::to_vec(&result).unwrap()).unwrap();

    // make sure parsing doesn't fail (since these are valid cases)
    let real_result = File::try_from(result).unwrap();
    let mut file = fs::File::create(file_result_path).unwrap();
    writeln!(file, "{:#?}", real_result).unwrap();

    println!("Done.");
}

#[test]
fn all_test_cases() {
    let dir = fs::read_dir("../../tests").unwrap();
    for subdir in dir {
        let dir = subdir.unwrap();
        let file_name = dir.file_name().into_string().unwrap();
        if let Some(file_name) = file_name.strip_suffix(".json") {
            if !file_name.ends_with("_result") {
                test_case(file_name);
            }
        }
    }
}
