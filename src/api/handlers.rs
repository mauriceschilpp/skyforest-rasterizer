use axum::{
    extract::Query,
    http::{StatusCode, header},
    response::Response,
    Json,
    body::Body,
};
use axum::extract::multipart::Multipart;
use std::time::Instant;
use std::io::Cursor;

use crate::{TiffReader, Result as RasterkitResult};
use super::models::*;

pub async fn get_coordinate_value(
    Query(req): Query<CoordinateRequest>,
) -> Result<Json<CoordinateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    match extract_single_value(&req.tiff_path, req.latitude, req.longitude, req.epsg) {
        Ok(value) => {
            let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;

            Ok(Json(CoordinateResponse {
                latitude: req.latitude,
                longitude: req.longitude,
                exposure_value: Some(value),
                execution_time_ms,
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to extract value: {}", e),
            }),
        )),
    }
}

pub async fn upload_csv(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    let mut csv_data: Option<Vec<u8>> = None;
    let mut tiff_path: Option<String> = None;
    let mut epsg: u16 = 4326;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "csv" => {
                csv_data = Some(field.bytes().await.unwrap_or_default().to_vec());
            }
            "tiff_path" => {
                tiff_path = Some(field.text().await.unwrap_or_default());
            }
            "epsg" => {
                if let Ok(text) = field.text().await {
                    epsg = text.parse().unwrap_or(4326);
                }
            }
            _ => {}
        }
    }

    let csv_data = csv_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing CSV file".to_string(),
            }),
        )
    })?;

    let tiff_path = tiff_path.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing tiff_path parameter".to_string(),
            }),
        )
    })?;

    match process_csv_batch_stream(&csv_data, &tiff_path, epsg, start) {
        Ok(stream_body) => {
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/csv")
                .header(header::CONTENT_DISPOSITION, "attachment; filename=\"exposure_results.csv\"")
                .body(stream_body)
                .unwrap())
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to process CSV: {}", e),
            }),
        )),
    }
}

fn extract_single_value(tiff_path: &str, latitude: f64, longitude: f64, source_epsg: u16) -> RasterkitResult<u8> {
    use crate::formats::tiff::geotiff::GeoInfo;
    use crate::projection::Coordinate;

    let mut reader = TiffReader::open(tiff_path)?;
    let tiff = reader.read()?;
    let ifd = tiff.main_ifd().ok_or_else(|| {
        crate::Error::InvalidFormat("No main IFD found".to_string())
    })?;

    let geo_info = GeoInfo::from_ifd(ifd, &mut reader)?
        .ok_or_else(|| crate::Error::InvalidFormat("Not a GeoTIFF".to_string()))?;

    let coord = Coordinate {
        x: longitude,
        y: latitude,
        z: 0.0,
    };

    let (pixel_x, pixel_y) = geo_info.transform_crs_to_pixel(coord, source_epsg)?;

    reader.read_pixel_value(ifd, pixel_x as u64, pixel_y as u64)
}

fn process_csv_batch_stream(
    csv_data: &[u8],
    tiff_path: &str,
    source_epsg: u16,
    start: Instant,
) -> RasterkitResult<Body> {
    use crate::formats::tiff::geotiff::GeoInfo;
    use crate::projection::Coordinate;

    let mut reader = TiffReader::open(tiff_path)?;
    let tiff = reader.read()?;
    let ifd = tiff.main_ifd().ok_or_else(|| {
        crate::Error::InvalidFormat("No main IFD found".to_string())
    })?;

    let geo_info = GeoInfo::from_ifd(ifd, &mut reader)?
        .ok_or_else(|| crate::Error::InvalidFormat("Not a GeoTIFF".to_string()))?;

    reader.enable_prefetch(ifd);

    let mut csv_reader = csv::Reader::from_reader(Cursor::new(csv_data));
    let mut points: Vec<CsvPoint> = Vec::new();

    for result in csv_reader.deserialize() {
        if let Ok(point) = result {
            points.push(point);
        }
    }

    let dims = ifd.dimensions().ok_or_else(|| {
        crate::Error::InvalidFormat("Missing dimensions".to_string())
    })?;

    let input_coords: Vec<Coordinate> = points.iter().map(|point| Coordinate {
        x: point.longitude,
        y: point.latitude,
        z: 0.0,
    }).collect();

    let pixel_coords = geo_info.transform_crs_to_pixel_batch(&input_coords, source_epsg)?;

    let coords: Vec<(u64, u64)> = pixel_coords.iter().map(|&(pixel_x, pixel_y)| {
        if pixel_x >= 0.0 && pixel_y >= 0.0
           && pixel_x < dims.width as f64 && pixel_y < dims.height as f64 {
            (pixel_x as u64, pixel_y as u64)
        } else {
            (u64::MAX, u64::MAX)
        }
    }).collect();

    let valid_indices: Vec<usize> = coords.iter().enumerate()
        .filter(|(_, c)| c.0 != u64::MAX)
        .map(|(i, _)| i)
        .collect();

    let valid_coords_only: Vec<(u64, u64)> = valid_indices.iter()
        .map(|&i| coords[i])
        .collect();

    let valid_values = reader.read_pixels_batch(ifd, &valid_coords_only)?;

    let mut values = vec![0u8; coords.len()];
    for (i, &original_idx) in valid_indices.iter().enumerate() {
        values[original_idx] = valid_values[i];
    }

    let execution_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    let successful = valid_coords_only.len();
    let pixels_per_second = (successful as f64 / start.elapsed().as_secs_f64()).round();

    let mut csv_output = String::with_capacity(points.len() * 40);

    csv_output.push_str("# Statistics\n");
    csv_output.push_str(&format!("# Total points: {}\n", points.len()));
    csv_output.push_str(&format!("# Successful: {}\n", successful));
    csv_output.push_str(&format!("# Failed: {}\n", points.len() - successful));
    csv_output.push_str(&format!("# Execution time: {:.2} ms\n", execution_time_ms));
    csv_output.push_str(&format!("# Pixels per second: {:.0}\n", pixels_per_second));

    // Check if any point has a name to determine header
    let has_names = points.iter().any(|p| p.name.is_some());
    if has_names {
        csv_output.push_str("latitude,longitude,name,exposure_value\n");
    } else {
        csv_output.push_str("latitude,longitude,exposure_value\n");
    }

    for (i, point) in points.iter().enumerate() {
        let exposure_value = if coords[i].0 == u64::MAX {
            "OUT_OF_BOUNDS"
        } else {
            &values[i].to_string()
        };

        if has_names {
            let name = point.name.as_deref().unwrap_or("");
            csv_output.push_str(&format!(
                "{},{},{},{}\n",
                point.latitude, point.longitude, name, exposure_value
            ));
        } else {
            csv_output.push_str(&format!(
                "{},{},{}\n",
                point.latitude, point.longitude, exposure_value
            ));
        }
    }

    Ok(Body::from(csv_output))
}
