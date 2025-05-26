//! # GeoTiff Writer Module
//!
//! This module provides functionality for writing Digital Elevation Model (DEM)
//! data, specifically from a [`DemData`] struct, into a GeoTiff raster file.
//!
//! The primary function, [`write_dem_to_geotiff`], handles the creation of the
//! GeoTiff file, setting up necessary geospatial metadata (like CRS, extent,
//! and resolution), and writing the elevation data. It uses the `geotiff` crate
//! for the underlying GeoTiff format manipulation.

use std::fs::File;
use crate::{DemData, DemMetadata};
use geotiff::writer::GeoTiffWriter;
use geotiff::geo_keys::{GeoKey, GeographicTypeGeoKey, ProjectedCSTypeGeoKey};
use log::{debug, warn, error};
use geotiff::values::Rational;

/// Writes the provided [`DemData`] (which includes metadata and elevation values)
/// to a GeoTiff file at the specified `output_path`.
///
/// This function configures the GeoTiff with essential geospatial information:
/// *   **Dimensions:** Image width and height from `dem_data.metadata`.
/// *   **Geotransform:** Sets the `ModelTiepointTag` (mapping the raster's top-left
///     corner (0,0) to geographic coordinates `x_min`, `y_max`) and `ModelPixelScaleTag`
///     (defining cell/pixel size).
/// *   **Coordinate Reference System (CRS):** If an EPSG code is provided in
///     `dem_data.metadata.crs` (e.g., "EPSG:4326"), it attempts to set the appropriate
///     GeoKeys (`GeographicTypeGeoKey` or `ProjectedCSTypeGeoKey` and `GTModelTypeGeoKey`).
///     For non-EPSG CRS strings, it falls back to using `GTCitationGeoKey`.
/// *   **No-Data Value:** If `dem_data.metadata.no_data_value` is `Some`, it's written
///     to the GeoTiff.
/// *   **Data Type:** Elevation data is written as 32-bit floating-point values (`f32`).
///
/// # Arguments
///
/// * `dem_data`: A reference to the [`DemData`] struct containing the DEM's metadata
///   and elevation grid.
/// * `output_path`: A string slice (`&str`) representing the file path where the
///   GeoTiff will be created.
///
/// # Returns
///
/// * `Ok(())` - If the GeoTiff file is successfully written.
/// * `Err(String)` - If any error occurs during file creation, GeoTiff parameter
///   setting, data writing, or finalization. The string contains a descriptive
///   error message. Detailed error information is also logged via the `log` crate.
pub fn write_dem_to_geotiff(dem_data: &DemData, output_path: &str) -> Result<(), String> {
    debug!("Starting GeoTiff writing process for output path: {}", output_path);
    let metadata = &dem_data.metadata;

    if let Some(mc) = &metadata.mesh_code {
        info!("DEM Metadata includes mesh_code: '{}'. This will not be written to the GeoTiff in the current version.", mc);
    } else {
        debug!("No mesh_code present in DEM Metadata.");
    }

    debug!("Attempting to create GeoTiff file: {}", output_path);
    let mut file = File::create(output_path)
        .map_err(|e| {
            let msg = format!("GeoTiff Write Error: Failed to create file at '{}': {}", output_path, e);
            error!("{}", msg);
            msg
        })?;

    let mut writer = GeoTiffWriter::new(&mut file);
    debug!("GeoTiffWriter initialized for path: {}", output_path);

    // 1. Set up GeoTiff parameters
    debug!("Setting GeoTiff dimensions: width={}, height={}", metadata.width, metadata.height);
    writer.set_image_width(metadata.width as u32);
    writer.set_image_height(metadata.height as u32);

    // Data Type: f32 (SampleFormat::IEEEFP, BitsPerSample: 32)
    // The `geotiff` crate's `add_image` method should infer this from the data type (f32),
    // but we can be explicit if needed or if there were options.
    // For f32, it usually means SampleFormat = 3 (IEEE floating point)
    // and BitsPerSample = 32. These are often defaults for f32 data.

    // 2. Geotransform (ModelTransformation)
    // This is typically set using ModelTiepointTag and ModelPixelScaleTag.
    // ModelTiepointTag: (0, 0, 0, x_min, y_max, 0.0)
    //   - Raster X, Y, Z (0,0,0) maps to Model X, Y, Z (x_min, y_max, 0.0)
    // ModelPixelScaleTag: (cell_size_x, cell_size_y, 0.0)
    //   - Note: cell_size_y is positive here. The writer/reader handles orientation.
    debug!("Setting ModelTiepointTag: [0.0, 0.0, 0.0, {}, {}, 0.0]", metadata.x_min, metadata.y_max);
    writer.set_geo_key(GeoKey::ModelTiepointTag, &[0.0, 0.0, 0.0, metadata.x_min, metadata.y_max, 0.0])
        .map_err(|e| {
            let msg = format!("GeoTiff Write Error: Failed to set ModelTiepointTag: {:?}", e);
            error!("{}", msg);
            msg
        })?;

    debug!("Setting ModelPixelScaleTag: [{}, {}, 0.0]", metadata.cell_size_x, metadata.cell_size_y);
    writer.set_geo_key(GeoKey::ModelPixelScaleTag, &[
        Rational::from(metadata.cell_size_x), 
        Rational::from(metadata.cell_size_y), 
        Rational::from(0.0)                   
    ]).map_err(|e| {
        let msg = format!("GeoTiff Write Error: Failed to set ModelPixelScaleTag: {:?}", e);
        error!("{}", msg);
        msg
    })?;

    // 3. Coordinate Reference System (CRS)
    if let Some(crs_str) = &metadata.crs {
        debug!("Processing CRS string: {}", crs_str);
        if crs_str.to_uppercase().starts_with("EPSG:") {
            if let Ok(epsg_code) = crs_str[5..].parse::<u16>() {
                debug!("Parsed EPSG code: {}", epsg_code);
                // Heuristic for classifying EPSG codes
                if epsg_code >= 4000 && epsg_code < 5000 { // Common range for geographic CRS
                    debug!("Setting GeographicTypeGeoKey to EPSG:{}", epsg_code);
                    writer.set_geo_key(GeoKey::GeographicTypeGeoKey, GeographicTypeGeoKey::Epsg(epsg_code))
                        .map_err(|e| {
                            let msg = format!("GeoTiff Write Error: Failed to set GeographicTypeGeoKey EPSG:{}: {:?}", epsg_code, e);
                            error!("{}", msg);
                            msg
                        })?;
                    writer.set_geo_key(GeoKey::GTModelTypeGeoKey, geotiff::geo_keys::ModelType::Geographic)
                         .map_err(|e| {
                            let msg = format!("GeoTiff Write Error: Failed to set GTModelTypeGeoKey (Geographic): {:?}",e);
                            error!("{}",msg);
                            msg
                        })?;
                } else { // Assume projected otherwise
                    debug!("Setting ProjectedCSTypeGeoKey to EPSG:{}", epsg_code);
                    writer.set_geo_key(GeoKey::ProjectedCSTypeGeoKey, ProjectedCSTypeGeoKey::Epsg(epsg_code))
                        .map_err(|e| {
                            let msg = format!("GeoTiff Write Error: Failed to set ProjectedCSTypeGeoKey EPSG:{}: {:?}", epsg_code, e);
                            error!("{}", msg);
                            msg
                        })?;
                     writer.set_geo_key(GeoKey::GTModelTypeGeoKey, geotiff::geo_keys::ModelType::Projected)
                        .map_err(|e| {
                            let msg = format!("GeoTiff Write Error: Failed to set GTModelTypeGeoKey (Projected): {:?}",e);
                            error!("{}",msg);
                            msg
                        })?;
                }
            } else {
                let msg = format!("GeoTiff Write Error: Failed to parse EPSG code from CRS string: '{}'. Using GTCitationGeoKey as fallback.", crs_str);
                warn!("{}", msg);
                writer.set_geo_key(GeoKey::GTCitationGeoKey, crs_str.as_str())
                    .map_err(|e| {
                        let cit_msg = format!("GeoTiff Write Error: Failed to set GTCitationGeoKey for '{}': {:?}", crs_str, e);
                        error!("{}", cit_msg);
                        cit_msg
                    })?;
            }
        } else {
            warn!("CRS string '{}' is not in 'EPSG:XXXX' format. Using GTCitationGeoKey as fallback.", crs_str);
            writer.set_geo_key(GeoKey::GTCitationGeoKey, crs_str.as_str())
                .map_err(|e| {
                    let msg = format!("GeoTiff Write Error: Failed to set GTCitationGeoKey for non-EPSG CRS '{}': {:?}", crs_str, e);
                    error!("{}", msg);
                    msg
                })?;
        }
    } else {
        warn!("No CRS information provided in metadata. GeoTiff will lack CRS details.");
    }

    // 4. No-data Value
    if let Some(no_data) = metadata.no_data_value {
        debug!("Setting no-data value: {}", no_data);
        writer.set_nodata_value(no_data.to_string().as_str())
            .map_err(|e| {
                let msg = format!("GeoTiff Write Error: Failed to set no-data value '{}': {:?}", no_data, e);
                error!("{}", msg);
                msg
            })?;
    } else {
        debug!("No no-data value provided in metadata.");
    }

    // 5. Write Raster Data
    debug!("Writing elevation data ({} values) as f32.", dem_data.elevation_values.len());
    writer.add_image("DEM Band 1", &dem_data.elevation_values, metadata.width as u32, metadata.height as u32)
        .map_err(|e| {
            let msg = format!("GeoTiff Write Error: Failed to write elevation data: {:?}", e);
            error!("{}", msg);
            msg
        })?;

    debug!("Finalizing GeoTiff writing process.");
    writer.write().map_err(|e| {
        let msg = format!("GeoTiff Write Error: Failed to finalize GeoTiff writing: {:?}", e);
        error!("{}", msg);
        msg
    })?;

    debug!("GeoTiff writing completed successfully for: {}", output_path);
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DemData, DemMetadata};
    use std::env;
    use std::fs;
    // It's good practice to ensure logging is available for tests if they use functions that log.
    // However, initializing env_logger in library tests can sometimes conflict if not handled carefully.
    // For this exercise, we'll assume it's handled or not strictly needed for these specific tests to pass.
    // fn init_logger() { let _ = env_logger::builder().is_test(true).try_init(); }
    use std::path::PathBuf;

    fn create_sample_dem_data() -> DemData {
        DemData {
            metadata: DemMetadata {
                width: 2,
                height: 2,
                x_min: 135.0,
                y_max: 35.01,
                cell_size_x: 0.005,
                cell_size_y: 0.005,
                no_data_value: Some(-9999.0),
                crs: Some("EPSG:4326".to_string()),
                mesh_code: Some("5339".to_string()), // Added mesh_code
            },
            elevation_values: vec![1.0, 2.0, 3.0, 4.0],
        }
    }

    #[test]
    fn test_write_dem_to_geotiff_basic() {
        // init_logger(); // If you want to see logs during test runs
        let dem_data = create_sample_dem_data();
        let mut temp_path = env::temp_dir();
        temp_path.push("test_output_basic.tif"); // Unique name
        let output_path_str = temp_path.to_str().unwrap();

        let result = write_dem_to_geotiff(&dem_data, output_path_str);
        assert!(result.is_ok(), "write_dem_to_geotiff failed: {:?}", result.err());

        // Check if file was created
        assert!(PathBuf::from(output_path_str).exists(), "Output GeoTiff file was not created.");

        // Clean up
        let _ = fs::remove_file(output_path_str); // Use _ to ignore result if cleanup fails
    }

    #[test]
    fn test_write_dem_to_geotiff_no_crs_no_nodata() {
        // init_logger();
        let mut dem_data = create_sample_dem_data();
        dem_data.metadata.crs = None;
        dem_data.metadata.no_data_value = None;
        dem_data.metadata.mesh_code = None; // Explicitly set mesh_code to None for this test

        let mut temp_path = env::temp_dir();
        temp_path.push("test_output_no_crs_nodata.tif"); // Unique name
        let output_path_str = temp_path.to_str().unwrap();

        let result = write_dem_to_geotiff(&dem_data, output_path_str);
        assert!(result.is_ok(), "write_dem_to_geotiff (no crs/nodata) failed: {:?}", result.err());

        assert!(PathBuf::from(output_path_str).exists(), "Output GeoTiff file (no crs/nodata) was not created.");

        let _ = fs::remove_file(output_path_str);
    }

    #[test]
    fn test_write_dem_to_geotiff_projected_crs() {
        // init_logger();
        let mut dem_data = create_sample_dem_data();
        dem_data.metadata.crs = Some("EPSG:32632".to_string()); // UTM Zone 32N
        // mesh_code will use the default from create_sample_dem_data()

        let mut temp_path = env::temp_dir();
        temp_path.push("test_output_projected.tif"); // Unique name
        let output_path_str = temp_path.to_str().unwrap();

        let result = write_dem_to_geotiff(&dem_data, output_path_str);
        assert!(result.is_ok(), "write_dem_to_geotiff (projected CRS) failed: {:?}", result.err());

        assert!(PathBuf::from(output_path_str).exists(), "Output GeoTiff file (projected CRS) was not created.");

        let _ = fs::remove_file(output_path_str);
    }
}
