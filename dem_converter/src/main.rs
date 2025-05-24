use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process;

// Main executable for the DEM Converter.
//
// This binary provides a command-line interface (CLI) to convert Digital Elevation Model (DEM)
// data from XML files to the GeoTiff raster format. It utilizes the `dem_converter` library
// for the core parsing and writing functionalities.
//
// The CLI accepts paths for the input XML file and the desired output GeoTiff file.
// It handles file operations, calls the library functions, and reports success or errors
// to the console. Logging can be controlled via the `RUST_LOG` environment variable.

// Use the library crate `dem_converter`
use log::{info, error, debug};
use dem_converter::xml_parser::parse_dem_xml;
use dem_converter::geotiff_writer::write_dem_to_geotiff;
// DemData is also needed if we explicitly type the result of parse_dem_xml
// use dem_converter::DemData;

/// Command-line arguments for the DEM to GeoTiff converter.
///
/// Utilizes `clap` for parsing and validation of arguments.
#[derive(Parser, Debug)]
#[clap(author = "The DEM Converter Team <info@example.com>", version = "0.1.0", about = "Converts Digital Elevation Model (DEM) XML files to GeoTiff format.", long_about = None)]
struct CliArgs {
    /// Specifies the file path to the input DEM XML.
    /// This file should contain DEM data, preferably in a GML-like structure.
    #[clap(short, long, value_parser)]
    input_xml_path: PathBuf,

    /// Specifies the file path where the output GeoTiff (.tif) file will be saved.
    /// If the file already exists, it will be overwritten.
    #[clap(short, long, value_parser)]
    output_geotiff_path: PathBuf,
}

/// Main entry point for the DEM converter CLI application.
///
/// Orchestrates the conversion process:
/// 1. Initializes the logger (`env_logger`).
/// 2. Parses command-line arguments using `CliArgs`.
/// 3. Reads the content of the input XML file.
/// 4. Calls the `parse_dem_xml` function from the `dem_converter` library.
/// 5. Calls the `write_dem_to_geotiff` function from the `dem_converter` library.
/// 6. Prints success or error messages to the console and log.
///
/// Exits with status code 1 on any critical error.
fn main() {
    env_logger::init(); // Initialize logger, configurable via RUST_LOG

    let args = CliArgs::parse();

    info!("Starting DEM conversion for input file: {}", args.input_xml_path.display());

    // 1. Read Input XML File
    debug!("Attempting to read XML file: {}", args.input_xml_path.display());
    let xml_content = match fs::read_to_string(&args.input_xml_path) {
        Ok(content) => {
            debug!("Successfully read XML file: {}", args.input_xml_path.display());
            content
        }
        Err(e) => {
            error!("Failed to read input XML file '{}': {}", args.input_xml_path.display(), e);
            eprintln!("Error: Could not read input file '{}'. Please check the path and permissions.", args.input_xml_path.display());
            process::exit(1);
        }
    };

    // 2. Call Parser
    debug!("Attempting to parse XML content from: {}", args.input_xml_path.display());
    let dem_data = match parse_dem_xml(&xml_content) {
        Ok(data) => {
            debug!("Successfully parsed XML content from: {}", args.input_xml_path.display());
            data
        }
        Err(e) => {
            error!("Failed to parse DEM XML from '{}': {}", args.input_xml_path.display(), e);
            eprintln!("Error: Could not parse XML data from '{}'. Ensure the file is a valid DEM XML.", args.input_xml_path.display());
            process::exit(1);
        }
    };

    // 3. Call Writer
    debug!("Preparing to write GeoTiff to: {}", args.output_geotiff_path.display());
    // Convert PathBuf to &str for the writer function
    let output_path_str = match args.output_geotiff_path.to_str() {
        Some(s) => s,
        None => {
            let err_msg = format!("Output path '{}' is not valid UTF-8.", args.output_geotiff_path.display());
            error!("{}", err_msg);
            eprintln!("Error: {}", err_msg);
            process::exit(1);
        }
    };

    debug!("Attempting to write DemData to GeoTiff file: {}", output_path_str);
    match write_dem_to_geotiff(&dem_data, output_path_str) {
        Ok(_) => {
            info!("Successfully converted '{}' to '{}'", args.input_xml_path.display(), args.output_geotiff_path.display());
            println!("Successfully created GeoTiff: {}", args.output_geotiff_path.display());
        }
        Err(e) => {
            error!("Failed to write GeoTiff to '{}': {}", args.output_geotiff_path.display(), e);
            eprintln!("Error: Could not write GeoTiff file to '{}'.", args.output_geotiff_path.display());
            process::exit(1);
        }
    }
}
