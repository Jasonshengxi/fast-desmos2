use bitflags::bitflags;
use glam::DVec2;
use std::fmt::{Debug, Formatter};

mod from_web;

#[rustfmt::skip]
pub fn from_web() {
    let address = 
        "https://www.desmos.com/calculator/toubpbklzp"; // Conway's GoL
    
    let file = File::try_from_direct_url(address).unwrap();
    println!("{file:#?}");
}

#[derive(Debug)]
pub struct File {
    pub settings: Settings,
    pub folders: Vec<Folder>,
    pub statements: Vec<Statement>,
    pub ticker: Option<Ticker>,
}

#[derive(Debug)]
pub struct Ticker {
    pub expr: String,
    pub freq: f64,
}

#[derive(Debug)]
pub struct Settings {
    pub show_graph_parts: ShowGraphParts,
    pub degree_mode: bool,
    pub cam_target: DVec2,
    pub zoom: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_graph_parts: ShowGraphParts::all(),
            degree_mode: false,
            cam_target: DVec2::ZERO,
            zoom: 0.1,
        }
    }
}

bitflags! {
    #[derive(Debug)]
    pub struct ShowGraphParts: u8 {
        const X_AXIS = 0b001;
        const Y_AXIS = 0b010;
        const GRID   = 0b100;
    }

    #[derive(Debug, Eq, PartialEq)]
    pub struct CanDragAxis: u8 {
        const BOTH = 0b11;
        const X_AXIS = 0b01;
        const Y_AXIS = 0b10;
        const NONE = 0b00;
    }
}

impl Default for CanDragAxis {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(PartialEq)]
pub struct PointSettings {
    pub show: bool,
    pub opacity: Option<String>,
    pub radius: Option<String>,
    pub drag_axis: CanDragAxis,
}

impl Default for PointSettings {
    fn default() -> Self {
        Self {
            show: true,
            opacity: None,
            drag_axis: Default::default(),
            radius: None,
        }
    }
}

impl Debug for PointSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if Self::default().eq(self) {
            f.debug_struct("PointSettings::Default").finish()
        } else {
            f.debug_struct("PointSettings")
                .field("show", &self.show)
                .field("opacity", &self.opacity)
                .field("radius", &self.radius)
                .field("drag_axis", &self.drag_axis)
                .finish()
        }
    }
}

#[derive(Debug)]
pub struct LineSettings {
    pub show: bool,
    pub opacity: Option<String>,
    pub width: Option<String>,
}

#[derive(Debug)]
pub struct FillSettings {
    pub show: bool,
    pub opacity: Option<String>,
}

#[derive(Debug)]
pub struct VisualSettings {
    pub point_settings: PointSettings,
    pub line_settings: LineSettings,
    pub fill_settings: FillSettings,
}

#[derive(Debug)]
pub struct Folder {
    pub title: String,
    pub collapsed: bool,
}

#[derive(Debug)]
pub struct Statement {
    pub folder_id: Option<usize>,
    pub expr: String,
    pub visual_settings: VisualSettings,
}
