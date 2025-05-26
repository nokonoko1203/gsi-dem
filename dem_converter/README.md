# DEM XML to GeoTiff Converter

## Description

`dem_converter` is a command-line tool and Rust library designed to convert Digital Elevation Model (DEM) data from XML files, conforming to a specific JPGIS (Japan Profile for Geographic Information Standards) GML structure, into the GeoTiff raster format. This tool was developed as part of a coding exercise and now adheres to a user-provided specification for the input XML.

## Build Instructions

To build the project, ensure you have Rust and Cargo installed.

1.  Clone the repository (if applicable).
2.  Navigate to the project directory: `cd dem_converter`
3.  Build the project:
    ```bash
    cargo build
    ```
4.  For an optimized executable, build in release mode:
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/dem_converter`.

## Usage Instructions (CLI)

The command-line interface allows you to convert an XML DEM file to a GeoTiff file.

### Syntax

```bash
target/release/dem_converter --input-xml-path <path_to_xml_file> --output-geotiff-path <path_to_output_tif_file>
```

Or using short arguments:

```bash
target/release/dem_converter -i <path_to_xml_file> -o <path_to_output_tif_file>
```

### Arguments

*   `-i, --input-xml-path <path_to_xml_file>`: Specifies the path to the input DEM XML file. This argument is required.
*   `-o, --output-geotiff-path <path_to_output_tif_file>`: Specifies the path where the output GeoTiff file will be saved. This argument is required.

### Logging

Logging can be enabled using the `RUST_LOG` environment variable. The logging level can be set to control verbosity.

Examples:
*   Show informational messages and above:
    ```bash
    RUST_LOG=info target/release/dem_converter -i input.xml -o output.tif
    ```
*   Show debug messages specifically for the `dem_converter` crate:
    ```bash
    RUST_LOG=dem_converter=debug target/release/dem_converter -i input.xml -o output.tif
    ```
*   Show all debug messages (very verbose):
    ```bash
    RUST_LOG=debug target/release/dem_converter -i input.xml -o output.tif
    ```

## Library Usage Example

The core conversion logic can also be used as a library in other Rust projects.

```rust
// Add to your Cargo.toml:
// dem_converter = { path = "path/to/dem_converter" } or from a registry if published.

// Example usage:
// use dem_converter::xml_parser::parse_dem_xml;
// use dem_converter::geotiff_writer::write_dem_to_geotiff;
// // DemData and DemMetadata structs also need to be in scope if used directly.
// // use dem_converter::{DemData, DemMetadata}; 
//
// fn main() -> Result<(), String> {
//     // Ensure DemData and DemMetadata are accessible if you are constructing them manually
//     // or directly interacting with the fields.
//     // Note: DemMetadata now includes an optional `mesh_code` field.
//
//     let xml_file_path = "path/to/your/input.xml";
//     let output_tiff_path = "path/to/your/output.tif";
//
//     // 1. Read XML content from a file
//     let xml_content = match std::fs::read_to_string(xml_file_path) {
//         Ok(content) => content,
//         Err(e) => {
//             return Err(format!("Failed to read XML file '{}': {}", xml_file_path, e));
//         }
//     };
//
//     // 2. Parse the XML content
//     // The parse_dem_xml function is part of the public API of the dem_converter library.
//     let dem_data = match parse_dem_xml(&xml_content) {
//         Ok(data) => data,
//         Err(e) => {
//             return Err(format!("Failed to parse DEM XML from '{}': {}", xml_file_path, e));
//         }
//     };
//
//     // 3. Write to GeoTiff
//     // The write_dem_to_geotiff function is also part of the public API.
//     match write_dem_to_geotiff(&dem_data, output_tiff_path) {
//         Ok(_) => {
//             println!("Successfully converted DEM XML to GeoTiff: {}", output_tiff_path);
//             Ok(())
//         }
//         Err(e) => {
//             Err(format!("Failed to write GeoTiff to '{}': {}", output_tiff_path, e))
//         }
//     }
// }
```

## XML Format Adherence

The XML parser in this tool is designed to process Digital Elevation Model (DEM) files that conform to a specific JPGIS (Japan Profile for Geographic Information Standards) GML (Geography Markup Language) structure. This structure is based on a provided specification and sample XML.

The parser expects the following key XML elements and structure:

*   **Root Element**: `<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset">`
    *   Must include the GML namespace (e.g., `xmlns:gml="http://www.opengis.net/gml/3.2"`).
*   **DEM Data Container**: `<DEM>` (within the default namespace)
    *   **Mesh Code (Optional)**: `<mesh>` - Contains the mesh code text (e.g., "533946").
    *   **Coordinate Reference System (CRS)**:
        *   Path: `<spatialReferenceInfo>/<SpatialReference>`
        *   Attribute: `system` on `<SpatialReference>` (e.g., `urn:ogc:def:crs:EPSG::6667`). The EPSG code is extracted from this URN.
    *   **Grid Definition**: `<gml:RectifiedGrid>`
        *   **Dimensions**: Defined by `<gml:limits>/<gml:GridEnvelope>`:
            *   `<gml:low>`: Must contain "0 0".
            *   `<gml:high>`: Contains two space-separated integers representing `columns-1` and `rows-1`. The parser calculates `width = (cols-1) + 1` and `height = (rows-1) + 1`.
        *   **Origin (Top-Left Corner)**: Defined by `<gml:origin>/<gml:Point>/<gml:pos>`:
            *   Contains two space-separated floating-point numbers. The parser interprets these as "latitude longitude" (Y X), corresponding to the `y_max` (northernmost edge) and `x_min` (westernmost edge) of the grid.
        *   **Cell Size/Resolution**: Defined by two `<gml:offsetVector>` elements:
            1.  The first `<gml:offsetVector>`: Its first numeric value is taken as `cell_size_x`.
            2.  The second `<gml:offsetVector>`: Its second numeric value is taken as `cell_size_y` (the absolute value is used, as cell size is stored positively).
    *   **Elevation Data**:
        *   Path: `<gml:Coverage>/<gml:rangeSet>/<gml:DataBlock>/<gml:tupleList>`
        *   The text content of `<gml:tupleList>` contains space-separated floating-point elevation values. These are read row by row.
    *   **No-Data Value**: The specific location for a no-data value tag was not detailed in the provided JPGIS GML structure. The parser retains some legacy logic to find `<gml:nilValues>` or `<swe:nilValues>`, but its effectiveness for the target format is uncertain. If no such tag is found, it's assumed all data points are valid or the no-data value is implicitly understood by the data consumer.

If your XML files deviate significantly from this structure, the parser may not function correctly.

## Extracted Metadata (Features)

The tool extracts the following key metadata from the XML and uses it to structure the GeoTiff:

*   **Grid Dimensions**: Width and Height of the DEM grid.
*   **Geographic Extent**: Origin coordinates (x_min, y_max) representing the top-left corner of the grid.
*   **Cell Resolution**: Cell size in X and Y directions.
*   **Coordinate Reference System (CRS)**: Parsed as an EPSG code string (e.g., "EPSG:6667").
*   **Mesh Code**: An optional identifier for the DEM tile or region, extracted from the `<mesh>` tag.
*   **Elevation Values**: The grid of elevation data.
*   **No-Data Value**: An optional value indicating missing data points (parsing for this is currently based on common GML tags like `<gml:nilValues>`, as it's not explicitly defined in the provided JPGIS structure).

## Error Handling

Errors encountered during file operations, XML parsing, or GeoTiff writing are reported to `stderr`. More detailed error information, including internal steps and warnings, is logged and can be viewed by setting the `RUST_LOG` environment variable (see Logging section).

## License

This project is intended as a coding exercise. You can consider it under the MIT License if you wish to use or modify it.
(No formal LICENSE file is included in this iteration).
