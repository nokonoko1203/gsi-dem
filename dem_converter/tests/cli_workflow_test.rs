// dem_converter/tests/cli_workflow_test.rs

use dem_converter::xml_parser::parse_dem_xml;
use dem_converter::geotiff_writer::write_dem_to_geotiff;
// DemData and DemMetadata are implicitly in scope via `dem_converter::*` but explicit for clarity
use dem_converter::{DemData, DemMetadata};

use std::env;
use std::fs;
use std::path::PathBuf;

/// Test XML string based on the new JPGIS/GML specification.
/// This sample represents a 2x2 grid.
/// - mesh: 533946
/// - CRS: EPSG:6677
/// - gml:high: "1 1" (means width=2, height=2 because it's cols-1, rows-1)
/// - gml:pos: "35.0 139.0" (y_max = 35.0, x_min = 139.0)
/// - offsetVector1: "0.00125 0.0" (cell_size_x = 0.00125)
/// - offsetVector2: "0.0 -0.0008333333333333334" (cell_size_y = abs value)
/// - tupleList: "10.1 10.2 10.5 10.6" (2x2 = 4 values)
const TEST_JPGIS_XML_VALID_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xsi:schemaLocation="http://fgd.gsi.go.jp/spec/2008/FGD_DatasetSpec.xsd"
    xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset"
    xmlns:gml="http://www.opengis.net/gml/3.2"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    gml:id="Dataset_DEM_Test_533946">
    <DEM gml:id="DEM_Test_533946_01">
        <mesh>533946</mesh>
        <spatialReferenceInfo>
            <SpatialReference system="urn:ogc:def:crs:EPSG::6677"/>
        </spatialReferenceInfo>
        <gml:RectifiedGrid gml:id="Grid_Test_533946_01" dimension="2">
            <gml:limits>
                <gml:GridEnvelope>
                    <gml:low>0 0</gml:low>
                    <gml:high>1 1</gml:high> <!-- width=2, height=2 -->
                </gml:GridEnvelope>
            </gml:limits>
            <gml:axisLabels>Y X</gml:axisLabels> <!-- Or i j, not directly used by parser -->
            <gml:origin>
                <gml:Point gml:id="P_Origin_Test_533946">
                    <gml:pos>35.0 139.0</gml:pos> <!-- y_max (latitude), x_min (longitude) -->
                </gml:Point>
            </gml:origin>
            <gml:offsetVector>0.00125 0.0</gml:offsetVector> <!-- cell_size_x from first value -->
            <gml:offsetVector>0.0 -0.0008333333333333334</gml:offsetVector> <!-- cell_size_y from second value (abs) -->
        </gml:RectifiedGrid>
        <gml:Coverage gml:id="Coverage_Test_533946_01">
            <gml:rangeSet>
                <gml:DataBlock>
                    <gml:rangeParameters/> <!-- This element is often empty or contains other metadata not currently parsed -->
                    <gml:tupleList>
                        10.1 10.2
                        10.5 10.6
                    </gml:tupleList>
                </gml:DataBlock>
            </gml:rangeSet>
        </gml:Coverage>
        <!-- No explicit no-data value tag is defined in the new provided structure -->
    </DEM>
</Dataset>
"#;

/// Test XML string based on the new JPGIS/GML specification, but without the optional <mesh> tag.
const TEST_JPGIS_XML_NO_MESH_CODE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xsi:schemaLocation="http://fgd.gsi.go.jp/spec/2008/FGD_DatasetSpec.xsd"
    xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset"
    xmlns:gml="http://www.opengis.net/gml/3.2"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    gml:id="Dataset_DEM_Test_NoMesh">
    <DEM gml:id="DEM_Test_NoMesh_01">
        <!-- <mesh>533946</mesh> --> <!-- MESH TAG OMITTED -->
        <spatialReferenceInfo>
            <SpatialReference system="urn:ogc:def:crs:EPSG::6677"/>
        </spatialReferenceInfo>
        <gml:RectifiedGrid gml:id="Grid_Test_NoMesh_01" dimension="2">
            <gml:limits>
                <gml:GridEnvelope>
                    <gml:low>0 0</gml:low>
                    <gml:high>1 1</gml:high>
                </gml:GridEnvelope>
            </gml:limits>
            <gml:axisLabels>Y X</gml:axisLabels>
            <gml:origin>
                <gml:Point gml:id="P_Origin_Test_NoMesh">
                    <gml:pos>35.0 139.0</gml:pos>
                </gml:Point>
            </gml:origin>
            <gml:offsetVector>0.00125 0.0</gml:offsetVector>
            <gml:offsetVector>0.0 -0.0008333333333333334</gml:offsetVector>
        </gml:RectifiedGrid>
        <gml:Coverage gml:id="Coverage_Test_NoMesh_01">
            <gml:rangeSet>
                <gml:DataBlock>
                    <gml:rangeParameters/>
                    <gml:tupleList>
                        10.1 10.2
                        10.5 10.6
                    </gml:tupleList>
                </gml:DataBlock>
            </gml:rangeSet>
        </gml:Coverage>
    </DEM>
</Dataset>
"#;


// Helper to create a unique temporary file path for GeoTiffs
fn temp_tiff_path(test_name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("test_integration_{}.tif", test_name)); 
    path
}

#[test]
fn test_full_conversion_workflow_new_spec() {
    // 1. Parse the XML
    let parse_result = parse_dem_xml(TEST_JPGIS_XML_VALID_SAMPLE);
    assert!(parse_result.is_ok(), "Integration Test (New Spec): XML parsing failed: {:?}", parse_result.err());
    let dem_data = parse_result.unwrap();

    // Assert parsed metadata based on TEST_JPGIS_XML_VALID_SAMPLE
    assert_eq!(dem_data.metadata.width, 2, "Width mismatch"); // from gml:high "1 1" -> 1+1 = 2
    assert_eq!(dem_data.metadata.height, 2, "Height mismatch"); // from gml:high "1 1" -> 1+1 = 2
    assert_eq!(dem_data.metadata.x_min, 139.0, "x_min mismatch"); // from gml:pos (second value)
    assert_eq!(dem_data.metadata.y_max, 35.0, "y_max mismatch"); // from gml:pos (first value)
    assert_eq!(dem_data.metadata.cell_size_x, 0.00125, "cell_size_x mismatch");
    assert_eq!(dem_data.metadata.cell_size_y, 0.0008333333333333334, "cell_size_y mismatch"); // abs value
    assert_eq!(dem_data.metadata.crs, Some("EPSG:6677".to_string()), "CRS mismatch");
    assert_eq!(dem_data.metadata.mesh_code, Some("533946".to_string()), "Mesh code mismatch");
    assert_eq!(dem_data.metadata.no_data_value, None, "No-data value should be None for new spec sample");
    
    assert_eq!(dem_data.elevation_values.len(), 4, "Elevation values count mismatch"); // 2x2 grid
    assert_eq!(dem_data.elevation_values, vec![10.1, 10.2, 10.5, 10.6], "Elevation values mismatch");

    // 2. Write to GeoTiff
    let output_path = temp_tiff_path("new_spec_valid");
    let output_path_str = output_path.to_str().expect("Temp path is not valid UTF-8");
    
    let _cleanup_guard = FileCleanupGuard::new(output_path.clone());

    let write_result = write_dem_to_geotiff(&dem_data, output_path_str);
    assert!(write_result.is_ok(), "Integration Test (New Spec): GeoTiff writing failed: {:?}", write_result.err());

    // 3. Verify File Existence and Size
    match fs::metadata(&output_path) {
        Ok(file_meta) => {
            assert!(file_meta.is_file(), "Integration Test (New Spec): Output path does not point to a file.");
            assert!(file_meta.len() > 0, "Integration Test (New Spec): Output file is empty.");
        }
        Err(e) => {
            panic!("Integration Test (New Spec): Failed to get metadata for output file '{}': {}", output_path.display(), e);
        }
    }
}

#[test]
fn test_conversion_workflow_no_mesh_code() {
    // 1. Parse the XML (sample without mesh code)
    let parse_result = parse_dem_xml(TEST_JPGIS_XML_NO_MESH_CODE);
    assert!(parse_result.is_ok(), "Integration Test (No Mesh): XML parsing failed: {:?}", parse_result.err());
    let dem_data = parse_result.unwrap();

    // Assert metadata, especially mesh_code
    assert_eq!(dem_data.metadata.width, 2, "Width mismatch (No Mesh)");
    assert_eq!(dem_data.metadata.height, 2, "Height mismatch (No Mesh)");
    assert_eq!(dem_data.metadata.crs, Some("EPSG:6677".to_string()), "CRS mismatch (No Mesh)");
    assert_eq!(dem_data.metadata.mesh_code, None, "Mesh code should be None"); // Key assertion
    assert_eq!(dem_data.elevation_values.len(), 4, "Elevation values count mismatch (No Mesh)");

    // 2. Write to GeoTiff
    let output_path = temp_tiff_path("no_mesh_code");
    let output_path_str = output_path.to_str().expect("Temp path is not valid UTF-8");
    let _cleanup_guard = FileCleanupGuard::new(output_path.clone());

    let write_result = write_dem_to_geotiff(&dem_data, output_path_str);
    assert!(write_result.is_ok(), "Integration Test (No Mesh): GeoTiff writing failed: {:?}", write_result.err());

    // 3. Verify File Existence
    assert!(fs::metadata(&output_path).is_ok(), "Integration Test (No Mesh): Output file does not exist or metadata check failed.");
}


/// A helper struct to ensure temporary files are cleaned up.
// This FileCleanupGuard remains the same.
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
