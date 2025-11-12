use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CoordinateRequest {
    pub latitude: f64,
    pub longitude: f64,
    pub tiff_path: String,
    #[serde(default = "default_epsg")]
    pub epsg: u16,
}

fn default_epsg() -> u16 {
    4326
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoordinateResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub exposure_value: Option<u8>,
    pub execution_time_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub total_points: usize,
    pub successful: usize,
    pub failed: usize,
    pub execution_time_ms: f64,
    pub pixels_per_second: f64,
    pub csv_data: String,
}

#[derive(Debug, Deserialize)]
pub struct CsvPoint {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CsvResult {
    pub latitude: f64,
    pub longitude: f64,
    pub exposure_value: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
