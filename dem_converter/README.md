# DEM XML to GeoTiff Converter

## Description

`dem_converter` is a command-line tool and Rust library designed to convert Digital Elevation Model (DEM) data from XML files (speculatively GML-based) into the GeoTiff raster format. This tool was developed as part of a coding exercise and currently makes assumptions about the input XML structure due to the unavailability of official schemas or sample files for Japanese Fundamental Geospatial Data (DEM) XML.

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

## Important Note on XML Format

The XML parser in this tool is currently **speculative**. Due to difficulties in accessing official Japanese Fundamental Geospatial Data (DEM) XML schemas or representative sample files, the parser has been designed based on common GML (Geography Markup Language) patterns often found in DEM data and general JPGIS documentation.

It primarily looks for the following GML tags (case-insensitive for local names, prefixes like `gml:` or `jps:` are common):

*   Grid Metadata and Structure:
    *   `<gml:GridCoverage>` or `<jps: jakościDemPoKryciuSiateczkowym>` (as root or main DEM element)
    *   `<gml:boundedBy>` containing `<gml:Envelope>`
    *   `<gml:Envelope>` with an `srsName` attribute for Coordinate Reference System (CRS).
    *   `<gml:lowerCorner>` and `<gml:upperCorner>` for the bounding box.
    *   `<gml:GridEnvelope>` or `<jps:GridEnvelope>` defining grid dimensions:
        *   `<gml:low>` (often "0 0")
        *   `<gml:high>` (e.g., "width-1 height-1" or "width height")
    *   `<gml:offsetVector>` or `<jps:offsetVector>` for cell size/pixel resolution (e.g., "cell_size_x cell_size_y").
    *   `srsName` attributes on various elements can define the CRS.
*   Elevation Data:
    *   `<gml:rangeSet>` containing `<gml:DataBlock>` (or similar structures like `<jps:coverage>`).
    *   `<gml:tupleList>` or `<jps:valueList>` for the actual elevation values, typically space-separated.
*   No-Data Value:
    *   `<gml:nilValues>` (often within `<gml:metadata>` or `swe:NilValues`) specifying the value used for missing data points.

The parser might not correctly interpret all variations or specific profiles of GSI DEM XML. If you have access to official GSI DEM XML samples or schema documents, the parser logic in `src/xml_parser.rs` would need to be updated accordingly.

## Error Handling

Errors encountered during file operations, XML parsing, or GeoTiff writing are reported to `stderr`. More detailed error information, including internal steps and warnings, is logged and can be viewed by setting the `RUST_LOG` environment variable (see Logging section).

## License

This project is intended as a coding exercise. You can consider it under the MIT License if you wish to use or modify it.
(No formal LICENSE file is included in this iteration).
