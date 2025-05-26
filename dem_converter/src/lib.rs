//! # DEM Converter Library
//!
//! `dem_converter` is a Rust library designed to facilitate the conversion of
//! Digital Elevation Model (DEM) data from XML files, specifically those conforming
//! to a JPGIS (Japan Profile for Geographic Information Standards) GML structure,
//! into the GeoTiff raster format.
//!
//! This library provides the core functionalities for:
//! - Parsing DEM XML data (adhering to the specified JPGIS GML format) to extract
//!   metadata and elevation values.
//! - Writing the extracted DEM data into a GeoTiff file, including relevant
//!   geospatial metadata.
//!
//! The primary components are the [`xml_parser`] module for handling XML input and
//! the [`geotiff_writer`] module for producing GeoTiff output. The main data
//! structures are [`DemMetadata`] and [`DemData`].
//!
//! ## XML Format Adherence
//! The XML parser (`xml_parser::parse_dem_xml`) is specifically tailored to the
//! JPGIS GML structure detailed in its documentation. While it aims for accuracy
//! based on this specification, deviations from the expected structure may lead to
//! parsing errors or incomplete data extraction.
//!
//! ## Intended Use
//! This library can be used as a dependency in other Rust projects that need to
//! perform DEM data conversions from the specified XML format, or as the backend
//! for command-line tools like the one provided in this project.

pub mod xml_parser;
pub mod geotiff_writer;

/// Represents the metadata associated with a Digital Elevation Model (DEM).
///
/// This struct holds essential information required to georeference and interpret
/// the grid of elevation values, extracted from the JPGIS GML XML format.
#[derive(Debug, Clone, PartialEq)]
pub struct DemMetadata {
    /// Number of columns in the grid.
    pub width: usize,
    /// Number of rows in the grid.
    pub height: usize,
/// Longitude or projected X coordinate of the **outer edge** of the westernmost column of cells
/// Longitude or projected X coordinate of the **outer edge** of the westernmost column of cells.
/// In the context of the parsed JPGIS GML, this corresponds to the second value in `<gml:pos>`
/// (interpreted as X, typically longitude) which defines the top-left corner of the grid.
    pub x_min: f64,
/// Latitude or projected Y coordinate of the **outer edge** of the northernmost row of cells.
/// In the context of the parsed JPGIS GML, this corresponds to the first value in `<gml:pos>`
/// (interpreted as Y, typically latitude) which defines the top-left corner of the grid.
    pub y_max: f64,
/// Pixel or cell dimension in the X-axis direction (e.g., in decimal degrees or meters).
/// This value is always positive. Derived from the first value of the first `<gml:offsetVector>`.
    pub cell_size_x: f64,
/// Pixel or cell dimension in the Y-axis direction (e.g., in decimal degrees or meters).
/// This value is always positive. Derived from the absolute of the second value of the second `<gml:offsetVector>`.
    pub cell_size_y: f64,
/// The specific floating-point value used to represent missing or void data points
/// within the `elevation_values` grid. If `None`, it's assumed all points are valid.
/// Note: The provided JPGIS GML specification did not explicitly define a no-data value tag;
/// this field might be populated by legacy logic if common GML no-data tags are found.
    pub no_data_value: Option<f32>,
/// A string representation of the Coordinate Reference System (CRS), typically an EPSG code.
/// Extracted from the `system` attribute of `<SpatialReference>` (e.g., "EPSG:6667").
/// If `None`, the CRS is unknown or was not successfully parsed.
    pub crs: Option<String>,
    /// Optional mesh code identifying the region or tile of the DEM.
    /// Extracted from the text content of the `<mesh>` tag within the `<DEM>` element.
    pub mesh_code: Option<String>,
}

/// Represents a complete Digital Elevation Model (DEM), including its
/// [`DemMetadata`] and the actual `elevation_values`.
#[derive(Debug, Clone, PartialEq)]
pub struct DemData {
    /// Metadata associated with the DEM.
    pub metadata: DemMetadata,
/// A flat vector storing the grid's elevation data, typically in row-major order
/// (all values for the first row, then all for the second, and so on).
/// The total number of elements must be equal to `metadata.width * metadata.height`.
/// The order of pixels usually corresponds to scanning from top-left to bottom-right.
    pub elevation_values: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dem_metadata_creation() {
        let metadata = DemMetadata {
            width: 100,
            height: 200,
            x_min: 0.0,
            y_max: 60.0,
            cell_size_x: 0.001,
            cell_size_y: 0.001,
            no_data_value: Some(-9999.0),
            crs: Some("EPSG:4326".to_string()),
            mesh_code: Some("5339".to_string()),
        };
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.crs, Some("EPSG:4326".to_string()));
        assert_eq!(metadata.mesh_code, Some("5339".to_string()));
    }

    #[test]
    fn dem_data_creation() {
        let metadata = DemMetadata {
            width: 2,
            height: 2,
            x_min: 0.0,
            y_max: 1.0,
            cell_size_x: 0.5,
            cell_size_y: 0.5,
            no_data_value: None,
            crs: None,
            mesh_code: None,
        };
        let elevation_values = vec![1.0, 2.0, 3.0, 4.0];
        let dem_data = DemData {
            metadata: metadata.clone(), // Use clone if metadata is used later
            elevation_values,
        };
        assert_eq!(dem_data.metadata.width, 2);
        assert_eq!(dem_data.elevation_values.len(), 4);
        assert_eq!(dem_data.elevation_values[0], 1.0);
    }
}
