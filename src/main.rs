use rasterkit::{TiffReader, GeoInfo, Result, Transformer, Coordinate, projection::epsg};

fn main() -> Result<()> {
    println!("rasterkit-v2 - TIFF File Info\n");

    let mut reader = TiffReader::open("exposure.tif")?;
    let tiff = reader.read()?;

    println!("{}", tiff);

    if let Some(ifd) = tiff.main_ifd() {
        println!("Data Type: {}",
            ifd.data_type()
                .map(|dt| dt.name())
                .unwrap_or("Unknown"));

        if let Some(geo_info) = GeoInfo::from_ifd(ifd, &mut reader)? {
            println!("{}", geo_info);

            if let Some(dims) = ifd.dimensions() {
                if let Some((min_x, min_y, max_x, max_y)) = geo_info.bounding_box(dims.width, dims.height) {
                    println!("  Bounding Box:");
                    println!("    Min: ({}, {})", min_x, min_y);
                    println!("    Max: ({}, {})", max_x, max_y);
                    println!("    Extent: {} x {}", max_x - min_x, max_y - min_y);
                }
            }

            println!("\n--- Testing Pixel Reading ---");

            println!("\n1. Reading pixel at (100, 100) [tile 0]:");
            match reader.read_pixel_value(ifd, 100, 100) {
                Ok(value) => println!("   Pixel value: {}", value),
                Err(e) => println!("   Error: {}", e),
            }

            println!("\n1b. Reading pixel at (1000, 1000) [different tile]:");
            match reader.read_pixel_value(ifd, 1000, 1000) {
                Ok(value) => println!("   Pixel value: {}", value),
                Err(e) => println!("   Error: {}", e),
            }

            println!("\n2. Reading pixel at coordinate (-12508904, 7373818) [Middle of Canada]:");
            match reader.read_pixel_at_coord(ifd, -12508904.0, 7373818.0) {
                Ok(value) => println!("   Pixel value: {}", value),
                Err(e) => println!("   Error: {}", e),
            }

            println!("\n3. Reading center pixel:");
            if let Some(dims) = ifd.dimensions() {
                let center_x = dims.width / 2;
                let center_y = dims.height / 2;
                match reader.read_pixel_value(ifd, center_x, center_y) {
                    Ok(value) => println!("   Center pixel ({}, {}): value = {}", center_x, center_y, value),
                    Err(e) => println!("   Error: {}", e),
                }
            }

            println!("\n4. Testing multiple coordinates across Canada (WGS84 -> Web Mercator):");

            let test_coords = vec![
                ("Edmonton, AB", 53.5461, -113.4938),
                ("Calgary, AB", 51.0447, -114.0719),
                ("Vancouver, BC", 49.2827, -123.1207),
                ("Toronto, ON", 43.6532, -79.3832),
                ("Montreal, QC", 45.5017, -73.5673),
                ("Winnipeg, MB", 49.8951, -97.1384),
                ("Halifax, NS", 44.6488, -63.5752),
                ("Yellowknife, NT", 62.4540, -114.3718),
            ];

            let transformer = Transformer::new(epsg::WGS84, epsg::WEB_MERCATOR)?;

            for (name, lat, lon) in test_coords {
                println!("\n   Testing: {} (lat={}, lon={})", name, lat, lon);
                let wgs84_coord = Coordinate::from_lonlat(lon, lat);

                match transformer.transform(wgs84_coord) {
                    Ok(web_mercator) => {
                        println!("     Web Mercator: x={:.2}, y={:.2}", web_mercator.x, web_mercator.y);

                        match geo_info.transform_crs_to_pixel(wgs84_coord, epsg::WGS84) {
                            Ok((pixel_x, pixel_y)) => {
                                println!("     Pixel: ({:.2}, {:.2})", pixel_x, pixel_y);

                                if pixel_x >= 0.0 && pixel_y >= 0.0 {
                                    let px = pixel_x as u64;
                                    let py = pixel_y as u64;

                                    if let Some(dims) = ifd.dimensions() {
                                        if px < dims.width && py < dims.height {
                                            match reader.read_pixel_value(ifd, px, py) {
                                                Ok(value) => println!("     Value: {}", value),
                                                Err(e) => println!("     Error: {}", e),
                                            }
                                        } else {
                                            println!("     Out of bounds ({}x{})", dims.width, dims.height);
                                        }
                                    }
                                } else {
                                    println!("     Negative pixel coordinates");
                                }
                            }
                            Err(e) => println!("     Transform error: {}", e),
                        }
                    }
                    Err(e) => println!("     Transformation error: {}", e),
                }
            }
        }
    }

    Ok(())
}
