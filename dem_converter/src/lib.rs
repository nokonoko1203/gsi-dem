//! # DEM Converter Library
//!
//! `dem_converter` is a Rust library designed to facilitate the conversion of
//! Digital Elevation Model (DEM) data from XML-based formats (primarily speculative
//! GML-like structures) into the GeoTiff raster format.
//!
//! This library provides the core functionalities for:
//! - Parsing DEM XML data to extract metadata and elevation values.
//! - Writing the extracted DEM data into a GeoTiff file.
//!
//! The primary components are the `xml_parser` module for handling XML input and
//! the `geotiff_writer` module for producing GeoTiff output. The main data
//! structures are `DemMetadata` and `DemData`.
//!
//! ## Speculative XML Parsing
//! Due to the unavailability of official schemas or sample files for certain DEM XML formats
//! (like the Japanese Fundamental Geospatial Data DEM XML), the XML parsing capabilities
//! are based on common GML patterns and might require adjustments for specific XML structures.
//!
//! ## Intended Use
//! This library can be used as a dependency in other Rust projects that need to
//! perform DEM data conversions, or as the backend for command-line tools.

pub mod xml_parser;
pub mod geotiff_writer;

/// Represents the metadata associated with a Digital Elevation Model (DEM).
///
/// This struct holds essential information required to georeference and interpret
/// the grid of elevation values.
#[derive(Debug, Clone, PartialEq)]
pub struct DemMetadata {
    /// Number of columns in the grid.
    pub width: usize,
    /// Number of rows in the grid.
    pub height: usize,
/// Longitude or projected X coordinate of the **outer edge** of the westernmost column of cells
/// (often the lower-left or upper-left corner of the grid, depending on `y_max` definition).
/// For example, for a grid cell whose center is at `x_min_center`, this `x_min` would be `x_min_center - cell_size_x / 2.0`.
    pub x_min: f64,
/// Latitude or projected Y coordinate of the **outer edge** of the northernmost row of cells
/// (often the upper-left or upper-right corner of the grid, depending on `x_min` definition).
/// For example, for a grid cell whose center is at `y_max_center`, this `y_max` would be `y_max_center + cell_size_y / 2.0`.
    pub y_max: f64,
/// Pixel or cell dimension in the X-axis direction (e.g., in decimal degrees or meters).
/// This value is always positive.
    pub cell_size_x: f64,
/// Pixel or cell dimension in the Y-axis direction (e.g., in decimal degrees or meters).
/// This value is always positive. Note that in some raster contexts (like GeoTiff geotransform),
/// a negative value might be used if the origin is top-left, but `DemMetadata` stores it as positive.
    pub cell_size_y: f64,
/// The specific floating-point value used to represent missing or void data points
/// within the `elevation_values` grid. If `None`, it's assumed all points are valid.
    pub no_data_value: Option<f32>,
/// A string representation of the Coordinate Reference System (CRS).
    /// This can be an EPSG code (e.g., "EPSG:4326" for WGS84),
/// a Well-Known Text (WKT) string, or other standard CRS identifiers.
/// If `None`, the CRS is unknown.
    pub crs: Option<String>,
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
        };
        assert_eq!(metadata.width, 100);
        assert_eq!(metadata.crs, Some("EPSG:4326".to_string()));
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
