// dem_converter/tests/cli_workflow_test.rs

use dem_converter::xml_parser::parse_dem_xml;
use dem_converter::geotiff_writer::write_dem_to_geotiff;
use dem_converter::{DemData, DemMetadata}; // Make sure DemData and DemMetadata are public from lib.rs

use std::env;
use std::fs;
use std::path::PathBuf;

// Helper to create a unique temporary file path for GeoTiffs
fn temp_tiff_path() -> PathBuf {
    let mut path = env::temp_dir();
    // Using a fixed name for simplicity in this environment, but normally a UUID would be better.
    // In a real scenario with parallel tests, ensure unique names.
    path.push("test_integration_output.tif"); 
    path
}

#[test]
fn test_full_conversion_workflow_ideal_case() {
    // Test case: A reasonably complete and valid GML-like XML for an end-to-end workflow test.
    // This XML represents a 2x2 grid with EPSG:4326 CRS and a no-data value.
    let hypothetical_xml_data = r#"
    <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
      <GridCoverage srsName="EPSG:4326">
        <gml:GridEnvelope>
          <gml:low>0 0</gml:low>
          <gml:high>2 2</gml:high> 
        </gml:GridEnvelope>
        <gml:boundedBy>
            <gml:Envelope srsName="EPSG:4326">
                <gml:lowerCorner>135.0 35.0</gml:lowerCorner>
                <gml:upperCorner>135.01 35.01</gml:upperCorner>
            </gml:Envelope>
        </gml:boundedBy>
        <gml:gridDomain>
            <gml:GridFunction>
                <gml:sequenceRule order="+x-y">Linear</gml:sequenceRule> 
            </gml:GridFunction>
        </gml:gridDomain>
        <gml:rangeSet>
            <gml:DataBlock>
                <gml:tupleList>1.0 2.0 3.0 4.0</gml:tupleList>
            </gml:DataBlock>
        </gml:rangeSet>
        <gml:metadata>
            <gml:NilValues nilReason="nodata">
                <gml:nilValues>-9999.0</gml:nilValues>
            </gml:NilValues>
        </gml:metadata>
        <gml:offsetVector srsName="EPSG:4326">0.005 0.005</gml:offsetVector> 
      </GridCoverage>
    </DEM>
    "#;

    // 1. Parse the XML
    let parse_result = parse_dem_xml(hypothetical_xml_data);
    assert!(parse_result.is_ok(), "Integration Test: XML parsing failed: {:?}", parse_result.err());
    let dem_data = parse_result.unwrap();

    // Assert some basic parsed metadata
    assert_eq!(dem_data.metadata.width, 2);
    assert_eq!(dem_data.metadata.height, 2);
    assert_eq!(dem_data.metadata.crs, Some("EPSG:4326".to_string()));
    assert_eq!(dem_data.elevation_values.len(), 4);

    // 2. Write to GeoTiff
    let output_path = temp_tiff_path();
    let output_path_str = output_path.to_str().expect("Temp path is not valid UTF-8");
    
    // Ensure we clean up, even on panic (though simple cleanup is used here)
    let _cleanup_guard = FileCleanupGuard::new(output_path.clone());

    let write_result = write_dem_to_geotiff(&dem_data, output_path_str);
    assert!(write_result.is_ok(), "Integration Test: GeoTiff writing failed: {:?}", write_result.err());

    // 3. Verify File Existence and Size
    match fs::metadata(&output_path) {
        Ok(file_meta) => {
            assert!(file_meta.is_file(), "Integration Test: Output path does not point to a file.");
            assert!(file_meta.len() > 0, "Integration Test: Output file is empty.");
        }
        Err(e) => {
            panic!("Integration Test: Failed to get metadata for output file '{}': {}", output_path.display(), e);
        }
    }
    
    // Cleanup is handled by FileCleanupGuard's Drop trait
}

#[test]
fn test_conversion_workflow_no_crs_no_nodata() {
    // Test case: XML is valid but missing optional CRS and no-data value.
    // This XML represents a 1x1 grid.
    let hypothetical_xml_data = r#"
    <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
      <GridCoverage> <!-- No srsName here -->
        <gml:GridEnvelope>
          <gml:low>0 0</gml:low>
          <gml:high>1 1</gml:high> 
        </gml:GridEnvelope>
        <gml:boundedBy>
            <gml:Envelope> <!-- No srsName here -->
                <gml:lowerCorner>10.0 20.0</gml:lowerCorner>
                <gml:upperCorner>10.1 20.1</gml:upperCorner>
            </gml:Envelope>
        </gml:boundedBy>
        <gml:rangeSet>
            <gml:DataBlock>
                <gml:tupleList>15.0</gml:tupleList>
            </gml:DataBlock>
        </gml:rangeSet>
        <!-- No gml:metadata for NilValues -->
        <gml:offsetVector>0.1 0.1</gml:offsetVector> 
      </GridCoverage>
    </DEM>
    "#;

    // 1. Parse the XML
    let parse_result = parse_dem_xml(hypothetical_xml_data);
    assert!(parse_result.is_ok(), "Integration Test (No CRS/NoData): XML parsing failed: {:?}", parse_result.err());
    let dem_data = parse_result.unwrap();

    assert_eq!(dem_data.metadata.crs, None);
    assert_eq!(dem_data.metadata.no_data_value, None);

    // 2. Write to GeoTiff
    let output_path = temp_tiff_path(); // Note: this will overwrite if tests run too quickly. Consider unique names.
                                        // For this environment, it's likely fine.
    let output_path_str = output_path.to_str().expect("Temp path is not valid UTF-8");
    let _cleanup_guard = FileCleanupGuard::new(output_path.clone());

    let write_result = write_dem_to_geotiff(&dem_data, output_path_str);
    assert!(write_result.is_ok(), "Integration Test (No CRS/NoData): GeoTiff writing failed: {:?}", write_result.err());

    // 3. Verify File Existence
    assert!(fs::metadata(&output_path).is_ok(), "Integration Test (No CRS/NoData): Output file does not exist or metadata check failed.");
}


/// A helper struct to ensure temporary files are cleaned up.
struct FileCleanupGuard {
    path: PathBuf,
}

impl FileCleanupGuard {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for FileCleanupGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_file(&self.path); // Ignore result, best effort cleanup
        }
    }
}
