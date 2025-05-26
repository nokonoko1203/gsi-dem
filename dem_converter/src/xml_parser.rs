//! # XML Parser Module
//!
//! This module is responsible for parsing Digital Elevation Model (DEM) data
//! from XML files that conform to a specific JPGIS (Japan Profile for Geographic
//! Information Standards) GML structure.
//!
//! The primary function [`parse_dem_xml`] attempts to extract metadata and elevation
//! values from the XML content based on a user-provided specification and sample XML.
//! While tailored to this specification, users should be aware that deviations from
//! this precise structure might lead to parsing errors.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use crate::{DemData, DemMetadata};
use log::{debug, warn, error};

/// Internal helper function to extract and unescape text content from within an XML element.
///
/// Reads text events until an `End` or `Empty` event is encountered for the current element.
///
/// # Arguments
/// * `reader` - A mutable reference to the `quick_xml::Reader`.
/// * `tag_name_for_error` - The name of the tag being processed, for error reporting.
///
/// # Returns
/// A `Result` containing the unescaped text content as a `String`, or an error message `String`
/// if reading/parsing fails or EOF is unexpectedly reached.
fn get_text_from_event(reader: &mut Reader<&[u8]>, tag_name_for_error: &[u8]) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut txt_buf = Vec::new();
    debug!("Attempting to extract text from current XML element: {:?}", String::from_utf8_lossy(tag_name_for_error));
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => txt_buf.extend_from_slice(e.unescape().as_ref()),
            Ok(Event::End(_)) | Ok(Event::Empty(_)) => break,
            Ok(Event::Eof) => {
                let err_msg = format!(
                    "XML Parse Error: Unexpected EOF while reading text content for tag <{}>.",
                    String::from_utf8_lossy(tag_name_for_error)
                );
                error!("{}", err_msg);
                return Err(err_msg);
            }
            Err(e) => {
                let err_msg = format!(
                    "XML Read Error: Error reading text content for tag <{}>: {}",
                    String::from_utf8_lossy(tag_name_for_error), e
                );
                error!("{}", err_msg);
                return Err(err_msg);
            }
            _ => (), 
        }
        buf.clear();
    }
    String::from_utf8(txt_buf).map_err(|e| {
        let err_msg = format!(
            "XML Parse Error: Failed to parse text content for tag <{}> as UTF-8: {}",
            String::from_utf8_lossy(tag_name_for_error), e
        );
        error!("{}", err_msg);
        err_msg
    })
}

/// Parses DEM (Digital Elevation Model) data from an XML string into a [`DemData`] struct.
///
/// This function is specifically designed to parse XML files conforming to the
/// JPGIS (Japan Profile for Geographic Information Standards) GML structure,
/// based on a user-provided specification.
///
/// ## Expected XML Structure:
///
/// The parser anticipates the following structure:
/// *   **Root Element**: `<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset">` (must include GML namespace, e.g., `xmlns:gml="http://www.opengis.net/gml/3.2"`).
/// *   **DEM Container**: `<DEM>` (default namespace).
///     *   **Mesh Code (Optional)**: `<mesh>` containing the mesh identifier.
///     *   **CRS Information**: `<spatialReferenceInfo>/<SpatialReference system="urn:ogc:def:crs:EPSG::XXXX"/>`.
///     *   **Grid Definition**: `<gml:RectifiedGrid>`
///         *   `<gml:limits>/<gml:GridEnvelope>`:
///             *   `<gml:low>`: Must be "0 0".
///             *   `<gml:high>`: Defines grid dimensions as `columns-1 rows-1`.
///         *   `<gml:origin>/<gml:Point>/<gml:pos>`: Defines the top-left corner coordinates (latitude longitude, e.g., "Y X").
///         *   Two `<gml:offsetVector>` elements:
///             1.  First for X-direction cell size (first value taken).
///             2.  Second for Y-direction cell size (second value taken, absolute value used).
///     *   **Elevation Data**: `<gml:Coverage>/<gml:rangeSet>/<gml:DataBlock>/<gml:tupleList>` containing space-separated elevation values.
/// *   **No-Data Value**: The parser includes legacy logic to search for common GML no-data tags like `<gml:nilValues>`,
///     as the provided JPGIS GML structure did not explicitly define one. This part may need adjustment
///     if a specific no-data representation for the target format is identified.
///
/// # Arguments
///
/// * `xml_content` - A string slice (`&str`) containing the XML data to be parsed.
///
/// # Returns
///
/// * `Ok(DemData)` - If parsing is successful, returns a `DemData` struct.
/// * `Err(String)` - If parsing fails due to structural deviations, missing critical data,
///   or value parsing errors. Detailed error information is also logged.
pub fn parse_dem_xml(xml_content: &str) -> Result<DemData, String> {
    debug!("Starting XML parsing process (New Specification) for input of length {}.", xml_content.len());
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();

    // --- Metadata components ---
    let mut crs_epsg: Option<u16> = None;
    let mut grid_low: Option<String> = None;
    let mut grid_high_parts: Option<(usize, usize)> = None;
    let mut origin_pos_parts: Option<(f64, f64)> = None; // (y_max, x_min)
    let mut offset_vectors: Vec<Vec<f64>> = Vec::new();
    let mut tuple_list_content: Option<String> = None;
    let mut parsed_mesh_code: Option<String> = None; // Renamed to avoid conflict if DemMetadata::mesh_code is brought into scope
    let mut no_data_value: Option<f32> = None;


    // --- State flags for parsing context ---
    let mut in_dem = false;
    let mut in_spatial_reference_info = false;
    let mut in_rectified_grid = false;
    let mut in_grid_limits = false;
    let mut in_grid_envelope = false;
    let mut in_origin = false;
    let mut in_origin_point = false;
    let mut in_coverage = false;
    let mut in_range_set = false;
    let mut in_data_block = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(bs)) => {
                let current_tag_name = bs.name().into_inner();
                debug!("Start Tag: {:?}", String::from_utf8_lossy(current_tag_name));

                if current_tag_name == b"DEM" { in_dem = true; }
                else if in_dem && current_tag_name == b"spatialReferenceInfo" { in_spatial_reference_info = true; }
                else if in_spatial_reference_info && current_tag_name == b"SpatialReference" {
                    for attr in bs.attributes() {
                        let attr = attr.map_err(|e| format!("XML Attribute Error: {}", e))?;
                        if attr.key.into_inner() == b"system" {
                            let val_str = String::from_utf8(attr.value.into_owned())
                                .map_err(|e| format!("XML Parse Error: CRS system attribute not UTF-8: {}", e))?;
                            debug!("Found SpatialReference system attribute: {}", val_str);
                            if let Some(code_str) = val_str.strip_prefix("urn:ogc:def:crs:EPSG::") {
                                crs_epsg = code_str.parse::<u16>().ok();
                                if crs_epsg.is_none() {
                                    warn!("Failed to parse EPSG code from system attribute: {}", val_str);
                                } else {
                                     debug!("Parsed EPSG code: {:?}", crs_epsg);
                                }
                            }
                        }
                    }
                }
                else if in_dem && current_tag_name == b"gml:RectifiedGrid" { in_rectified_grid = true; }
                else if in_rectified_grid && current_tag_name == b"gml:limits" { in_grid_limits = true; }
                else if in_grid_limits && current_tag_name == b"gml:GridEnvelope" { in_grid_envelope = true; }
                else if in_grid_envelope && current_tag_name == b"gml:low" {
                    let text = get_text_from_event(&mut reader, current_tag_name)?;
                    debug!("gml:low content: {}", text);
                    if text.trim() != "0 0" {
                        return Err(format!("XML Structure Error: <gml:low> must be '0 0', found '{}'", text));
                    }
                    grid_low = Some(text);
                }
                else if in_grid_envelope && current_tag_name == b"gml:high" {
                    let text = get_text_from_event(&mut reader, current_tag_name)?;
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    if parts.len() == 2 {
                        if let (Ok(c), Ok(r)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            grid_high_parts = Some((c, r));
                            debug!("Parsed gml:high cols-1: {}, rows-1: {}", c, r);
                        } else {
                            warn!("Failed to parse numeric values from <gml:high>: {}", text);
                        }
                    } else {
                        warn!("Unexpected format for <gml:high>: {}. Expected 'cols-1 rows-1'.", text);
                    }
                }
                else if in_rectified_grid && current_tag_name == b"gml:origin" { in_origin = true; }
                else if in_origin && current_tag_name == b"gml:Point" { in_origin_point = true; }
                else if in_origin_point && current_tag_name == b"gml:pos" {
                    let text = get_text_from_event(&mut reader, current_tag_name)?;
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    if parts.len() == 2 {
                         if let (Ok(y_val), Ok(x_val)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                            origin_pos_parts = Some((y_val, x_val)); // y_max, x_min
                            debug!("Parsed gml:pos y_max: {}, x_min: {}", y_val, x_val);
                        } else {
                            warn!("Failed to parse numeric values from <gml:pos>: {}", text);
                        }
                    } else {
                        warn!("Unexpected format for <gml:pos>: {}. Expected 'lat lon'.", text);
                    }
                }
                else if in_rectified_grid && current_tag_name == b"gml:offsetVector" {
                    let text = get_text_from_event(&mut reader, current_tag_name)?;
                    let parts: Vec<f64> = text.split_whitespace()
                                               .filter_map(|s| s.parse::<f64>().ok())
                                               .collect();
                    if !parts.is_empty() {
                        debug!("Parsed gml:offsetVector content: {:?}", parts);
                        offset_vectors.push(parts);
                    } else {
                        warn!("Failed to parse numeric values from <gml:offsetVector>: {}", text);
                    }
                }
                else if in_dem && current_tag_name == b"gml:Coverage" { in_coverage = true; }
                else if in_coverage && current_tag_name == b"gml:rangeSet" { in_range_set = true; }
                else if in_range_set && current_tag_name == b"gml:DataBlock" { in_data_block = true; }
                else if in_data_block && current_tag_name == b"gml:tupleList" {
                    tuple_list_content = Some(get_text_from_event(&mut reader, current_tag_name)?);
                    debug!("Captured gml:tupleList content (first 100 chars): '{}'", tuple_list_content.as_ref().unwrap_or(&"".to_string()).chars().take(100).collect::<String>());
                }
                else if in_dem && current_tag_name == b"mesh" {
                    parsed_mesh_code = Some(get_text_from_event(&mut reader, current_tag_name)?);
                    debug!("Captured mesh code: {}", parsed_mesh_code.as_ref().unwrap_or(&"".to_string()));
                }
                else if current_tag_name == b"gml:nilValues" || current_tag_name == b"swe:nilValues" {
                    let text = get_text_from_event(&mut reader, current_tag_name)?;
                    no_data_value = text.trim().parse().ok();
                    debug!("Parsed no_data_value (legacy attempt): {:?}", no_data_value);
                }

            }
            Ok(Event::End(bs)) => {
                let current_tag_name = bs.name().into_inner();
                debug!("End Tag: {:?}", String::from_utf8_lossy(current_tag_name));

                if current_tag_name == b"DEM" { in_dem = false; }
                else if current_tag_name == b"spatialReferenceInfo" { in_spatial_reference_info = false; }
                else if current_tag_name == b"gml:RectifiedGrid" { in_rectified_grid = false; }
                else if current_tag_name == b"gml:limits" { in_grid_limits = false; }
                else if current_tag_name == b"gml:GridEnvelope" { in_grid_envelope = false; }
                else if current_tag_name == b"gml:origin" { in_origin = false; }
                else if current_tag_name == b"gml:Point" { in_origin_point = false; }
                else if current_tag_name == b"gml:Coverage" { in_coverage = false; }
                else if current_tag_name == b"gml:rangeSet" { in_range_set = false; }
                else if current_tag_name == b"gml:DataBlock" { in_data_block = false; }
            }
            Ok(Event::Text(_)) => { /* Text events are handled by get_text_from_event */ }
            Ok(Event::Eof) => {
                debug!("Reached end of XML document.");
                break;
            }
            Err(e) => {
                let err_msg = format!("XML Read Error: Error at position {}: {:?}", reader.buffer_position(), e);
                error!("{}", err_msg);
                return Err(err_msg);
            }
            _ => (), 
        }
        buf.clear();
    }

    debug!("Finished parsing XML elements. Proceeding to data validation and construction.");
    // parsed_mesh_code (Option<String>) is already populated from parsing if the <mesh> tag was found.
    // Logging of mesh_code presence or absence is handled by debug statements during parsing.

    // --- Post-processing and Validation (New Specification) ---
    let final_crs = crs_epsg.map(|code| format!("EPSG:{}", code));
    if final_crs.is_none() {
         warn!("CRS information (urn:ogc:def:crs:EPSG::XXXX from <SpatialReference system=...>) not found or failed to parse.");
    }

    let (cols_minus_1, rows_minus_1) = grid_high_parts.ok_or_else(|| {
        let msg = "XML Parse Error: Grid dimensions (<gml:high> within <gml:GridEnvelope>) are missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_width = cols_minus_1 + 1;
    let final_height = rows_minus_1 + 1;

    let (parsed_y_max, parsed_x_min) = origin_pos_parts.ok_or_else(|| {
        let msg = "XML Parse Error: Origin position (<gml:pos> within <gml:origin>/<gml:Point>) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;

    if offset_vectors.len() < 2 {
        return Err("XML Structure Error: Expected at least two <gml:offsetVector> elements for cell size.".to_string());
    }
    let final_cell_size_x = offset_vectors[0].get(0).copied().ok_or_else(|| {
        "XML Parse Error: First <gml:offsetVector> is missing the first value for cell_size_x.".to_string()
    })?;
    let final_cell_size_y = offset_vectors[1].get(1).copied().ok_or_else(|| {
        "XML Parse Error: Second <gml:offsetVector> is missing the second value for cell_size_y.".to_string()
    })?.abs(); 

    if grid_low.is_none() {
         return Err("XML Structure Error: <gml:low> element is missing.".to_string());
    }

    // --- Construct DemMetadata ---
    debug!("Constructing DemMetadata from parsed values.");
    let metadata = DemMetadata {
        width: final_width,
        height: final_height,
        x_min: parsed_x_min,
        y_max: parsed_y_max,
        cell_size_x: final_cell_size_x,
        cell_size_y: final_cell_size_y,
        no_data_value,
        crs: final_crs,
        mesh_code: parsed_mesh_code, 
    };
    debug!("DemMetadata constructed: {:?}", metadata);

    // --- Parse Elevation Values ---
    debug!("Parsing elevation values string.");
    let elevation_values_str_content = tuple_list_content
        .ok_or_else(|| {
            let msg = "XML Parse Error: Elevation data string (<gml:tupleList>) is missing.".to_string();
            error!("{}", msg);
            msg
        })?;

    let mut elevation_values: Vec<f32> = Vec::new();
    let cleaned_values_str = elevation_values_str_content.replace(",", " "); 
    debug!("Cleaned elevation string (first 100 chars): '{}'", cleaned_values_str.chars().take(100).collect::<String>());

    for val_str in cleaned_values_str.split_whitespace() {
        match val_str.trim().parse::<f32>() {
            Ok(val) => elevation_values.push(val),
            Err(parse_err) => {
                let msg = format!("XML Parse Error: Failed to parse elevation value '{}': {}", val_str, parse_err);
                error!("{}", msg);
                return Err(msg);
            }
        }
    }
    debug!("Successfully parsed {} elevation values.", elevation_values.len());

    if elevation_values.len() != metadata.width * metadata.height {
        let msg = format!(
            "Data Integrity Error: Mismatch between expected number of elevation values ({}, from width {} x height {}) and parsed values ({}).",
            metadata.width * metadata.height,
            metadata.width,
            metadata.height,
            elevation_values.len()
        );
        error!("{}", msg);
        return Err(msg);
    }

    debug!("XML parsing completed successfully.");
    Ok(DemData {
        metadata,
        elevation_values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DemMetadata; // Ensure DemMetadata is in scope for assertions

    const VALID_JPGIS_SAMPLE_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset"
    xmlns:gml="http://www.opengis.net/gml/3.2"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xmlns:xlink="http://www.w3.org/1999/xlink"
    xsi:schemaLocation="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset FGD_Dataset.xsd"
    gml:id="Dataset_1">
    <DEM>
        <mesh>533926</mesh>
        <spatialReferenceInfo>
            <SpatialReference system="urn:ogc:def:crs:EPSG::6667"/>
        </spatialReferenceInfo>
        <gml:RectifiedGrid gml:id="Grid_533926" dimension="2">
            <gml:limits>
                <gml:GridEnvelope>
                    <gml:low>0 0</gml:low>
                    <gml:high>224 149</gml:high> <!-- width=225, height=150 -->
                </gml:GridEnvelope>
            </gml:limits>
            <gml:axisLabels>Lat Long</gml:axisLabels> <!-- Or i j, not directly used by parser -->
            <gml:origin>
                <gml:Point gml:id="P_533926">
                    <gml:pos>36.1666666666667 139.833333333333</gml:pos> <!-- y_max (lat), x_min (lon) -->
                </gml:Point>
            </gml:origin>
            <gml:offsetVector>0.0 0.000104166666666667</gml:offsetVector> <!-- cell_size_x (lon-spacing) -->
            <gml:offsetVector>-0.0000833333333333333 0.0</gml:offsetVector> <!-- cell_size_y (lat-spacing, negative) -->
        </gml:RectifiedGrid>
        <gml:Coverage>
            <gml:rangeSet>
                <gml:DataBlock>
                    <gml:tupleList>
                        10.0 10.1 10.2 10.3 10.4
                        11.0 11.1 11.2 11.3 11.4
                        12.0 12.1 12.2 12.3 12.4
                    </gml:tupleList> <!-- Example: 3 rows, 5 columns. Actual sample would have 225*150 values -->
                </gml:DataBlock>
            </gml:rangeSet>
        </gml:Coverage>
        <!-- No explicit no-data value in this sample structure -->
    </DEM>
</Dataset>
    "#;

    // Simplified version for most tests to keep them concise, focusing on structure not full data.
    // Width=3, Height=2 based on gml:high 2 1
    fn build_test_xml(
        crs_system_attr: &str,
        gml_high_content: &str,
        gml_pos_content: &str,
        offset_vector1_content: &str,
        offset_vector2_content: &str,
        tuple_list_content: &str,
        mesh_content: Option<&str>,
        omit_dem_tag: bool,
        omit_spatial_ref_info: bool,
        omit_rectified_grid: bool,
        omit_limits: bool,
        omit_grid_envelope: bool,
        omit_gml_low: bool,
        omit_gml_high: bool,
        omit_origin: bool,
        omit_origin_point: bool,
        omit_pos: bool,
        omit_offset_vector1: bool,
        omit_offset_vector2: bool,
        omit_coverage: bool,
        omit_range_set: bool,
        omit_data_block: bool,
        omit_tuple_list: bool
    ) -> String {
        let mesh_tag = mesh_content.map_or("".to_string(), |m| format!("<mesh>{}</mesh>", m));
        
        let dem_content = if omit_dem_tag { "".to_string() } else {
            format!(r#"
            <DEM>
                {mesh_tag}
                {spatial_ref_info}
                {rectified_grid}
                {coverage}
            </DEM>
            "#,
            mesh_tag = mesh_tag,
            spatial_ref_info = if omit_spatial_ref_info { "".to_string() } else {
                format!(r#"<spatialReferenceInfo><SpatialReference system="{}"/></spatialReferenceInfo>"#, crs_system_attr)
            },
            rectified_grid = if omit_rectified_grid { "".to_string() } else {
                format!(r#"
                <gml:RectifiedGrid gml:id="TestGrid" dimension="2">
                    {limits}
                    {origin}
                    {offset_vector1}
                    {offset_vector2}
                </gml:RectifiedGrid>
                "#,
                limits = if omit_limits { "".to_string() } else {
                    format!(r#"<gml:limits>{grid_envelope}</gml:limits>"#,
                        grid_envelope = if omit_grid_envelope { "".to_string() } else {
                            format!(r#"<gml:GridEnvelope>{gml_low}{gml_high}</gml:GridEnvelope>"#,
                                gml_low = if omit_gml_low { "".to_string() } else { "<gml:low>0 0</gml:low>" },
                                gml_high = if omit_gml_high { "".to_string() } else { format!("<gml:high>{}</gml:high>", gml_high_content) }
                            )
                        }
                    )
                },
                origin = if omit_origin { "".to_string() } else {
                    format!(r#"<gml:origin>{point}</gml:origin>"#,
                        point = if omit_origin_point { "".to_string() } else {
                            format!(r#"<gml:Point gml:id="TestOrigin">{pos}</gml:Point>"#,
                                pos = if omit_pos { "".to_string() } else { format!("<gml:pos>{}</gml:pos>", gml_pos_content) }
                            )
                        }
                    )
                },
                offset_vector1 = if omit_offset_vector1 { "".to_string() } else { format!("<gml:offsetVector>{}</gml:offsetVector>", offset_vector1_content) },
                offset_vector2 = if omit_offset_vector2 { "".to_string() } else { format!("<gml:offsetVector>{}</gml:offsetVector>", offset_vector2_content) }
                )
            },
            coverage = if omit_coverage { "".to_string() } else {
                format!(r#"<gml:Coverage>{range_set}</gml:Coverage>"#,
                    range_set = if omit_range_set { "".to_string() } else {
                        format!(r#"<gml:rangeSet>{data_block}</gml:rangeSet>"#,
                            data_block = if omit_data_block { "".to_string() } else {
                                format!(r#"<gml:DataBlock>{tuple_list}</gml:DataBlock>"#,
                                    tuple_list = if omit_tuple_list { "".to_string() } else { format!("<gml:tupleList>{}</gml:tupleList>", tuple_list_content) }
                                )
                            }
                        )
                    }
                )
            })
        };

        format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset" xmlns:gml="http://www.opengis.net/gml/3.2">
    {}
</Dataset>"#, dem_content)
    }
    
    // Default builder for a minimal valid structure
    fn build_minimal_valid_xml(
        crs: &str, high: &str, pos: &str, ov1: &str, ov2: &str, tuples: &str
    ) -> String {
        build_test_xml(crs, high, pos, ov1, ov2, tuples, Some("5339"),
            false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false
        )
    }


    #[test]
    fn test_parse_valid_jpgis_sample_simplified() {
        // Using a simplified structure based on the provided sample, with fewer data points.
        // gml:high "2 1" means width=3, height=2. (cols-1, rows-1)
        // gml:pos "36.0 -139.0" means y_max=36.0, x_min=-139.0
        // offsetVector1 "0.0 0.1" means cell_size_x = 0.0
        // offsetVector2 "-0.1 0.0" means cell_size_y = 0.1 (abs(-0.1))
        // tupleList: 6 values for 3x2 grid
        let xml_data = build_minimal_valid_xml(
            "urn:ogc:def:crs:EPSG::6667",
            "2 1", // width=3, height=2
            "36.0 139.0", // y_max=36.0, x_min=139.0
            "0.001 0.0",  // cell_x = 0.001 (from first value)
            "0.0 -0.001", // cell_y = abs(-0.001) = 0.001 (from second value)
            "1.0 2.0 3.0 4.0 5.0 6.0"
        );

        let result = parse_dem_xml(&xml_data);
        assert!(result.is_ok(), "Parsing valid simplified JPGIS sample failed: {:?}", result.err());
        let dem_data = result.unwrap();

        assert_eq!(dem_data.metadata.width, 3, "Width mismatch");
        assert_eq!(dem_data.metadata.height, 2, "Height mismatch");
        assert_eq!(dem_data.metadata.x_min, 139.0, "x_min mismatch");
        assert_eq!(dem_data.metadata.y_max, 36.0, "y_max mismatch");
        assert_eq!(dem_data.metadata.cell_size_x, 0.001, "cell_size_x mismatch");
        assert_eq!(dem_data.metadata.cell_size_y, 0.001, "cell_size_y mismatch");
        assert_eq!(dem_data.metadata.crs, Some("EPSG:6667".to_string()), "CRS mismatch");
        assert_eq!(dem_data.metadata.no_data_value, None, "no_data_value should be None as not in new spec"); // Assuming no_data_value is not part of this new spec or found.
        
        assert_eq!(dem_data.elevation_values.len(), 6, "Elevation values count mismatch");
        assert_eq!(dem_data.elevation_values, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "Elevation values mismatch");
    }

    #[test]
    fn test_error_missing_dem_tag() {
        let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false);
        let result = parse_dem_xml(&xml);
        assert!(result.is_err(), "Should fail if <DEM> is missing");
        // A specific error message might be tricky if the main loop doesn't even start.
        // For now, just checking it's an error. A more robust check would be if a specific "DEM not found" type error is added.
        // This test will likely fail if the parser assumes <DEM> must exist from the get-go.
        // Current logic implies it will fail at `grid_high_parts.ok_or_else` because `in_dem` will never be true.
        assert!(result.err().unwrap().contains("Grid dimensions"));
    }
    
    #[test]
    fn test_error_missing_spatial_reference_info() {
        let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            false, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false);
        let result = parse_dem_xml(&xml);
        assert!(result.is_ok(), "Missing spatialReferenceInfo should be a warning, not an error, CRS will be None. Res: {:?}", result.err());
        assert_eq!(result.unwrap().metadata.crs, None);
    }

    #[test]
    fn test_error_missing_rectified_grid() {
        let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            false, false, true, false, false, false, false, false, false, false, false, false, false, false, false, false);
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Grid dimensions")); // Fails because gml:high is not found
    }
    
    #[test]
    fn test_error_invalid_gml_low() {
        let xml_data = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xmlns="http://fgd.gsi.go.jp/spec/2008/FGD_Dataset" xmlns:gml="http://www.opengis.net/gml/3.2">
<DEM>
    <spatialReferenceInfo><SpatialReference system="urn:ogc:def:crs:EPSG::6667"/></spatialReferenceInfo>
    <gml:RectifiedGrid gml:id="TestGrid" dimension="2">
        <gml:limits><gml:GridEnvelope>
            <gml:low>1 1</gml:low> <!-- Invalid -->
            <gml:high>2 1</gml:high>
        </gml:GridEnvelope></gml:limits>
        <gml:origin><gml:Point gml:id="P"><gml:pos>36.0 139.0</gml:pos></gml:Point></gml:origin>
        <gml:offsetVector>0.001 0.0</gml:offsetVector>
        <gml:offsetVector>0.0 -0.001</gml:offsetVector>
    </gml:RectifiedGrid>
    <gml:Coverage><gml:rangeSet><gml:DataBlock><gml:tupleList>1 2 3 4 5 6</gml:tupleList></gml:DataBlock></gml:rangeSet></gml:Coverage>
</DEM>
</Dataset>"#;
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err(), "Should fail for invalid gml:low");
        assert!(result.err().unwrap().contains("<gml:low> must be '0 0'"));
    }

    #[test]
    fn test_error_missing_offset_vectors() {
         let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            false, false, false, false, false, false, false, false, false, false, true, true, false, false, false, false); // omit both offset vectors
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Expected at least two <gml:offsetVector>"));
    }

    #[test]
    fn test_error_missing_one_offset_vector() {
         let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            false, false, false, false, false, false, false, false, false, false, false, true, false, false, false, false); // omit second offset vector
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Expected at least two <gml:offsetVector>"));
    }
    
    #[test]
    fn test_error_malformed_gml_high() {
        let xml = build_minimal_valid_xml("urn:ogc:def:crs:EPSG::6667", "2 non_numeric", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6");
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        // This might manifest as a general "Grid dimensions missing" if parsing `Ok(c), Ok(r)` fails and `grid_high_parts` remains None.
        // A more specific error message could be "Failed to parse numeric values from <gml:high>" if we check the `warn!` condition.
        assert!(result.err().unwrap().contains("Grid dimensions")); 
    }

    #[test]
    fn test_error_malformed_offset_vector_value() {
        let xml = build_minimal_valid_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "non_numeric 0.0", "0.0 -0.1", "1 2 3 4 5 6");
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("First <gml:offsetVector> is missing the first value"));
    }

    #[test]
    fn test_error_malformed_tuple_list_non_numeric() {
        let xml = build_minimal_valid_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 non_numeric 4 5 6");
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Failed to parse elevation value 'non_numeric'"));
    }

    #[test]
    fn test_error_elevation_count_mismatch() {
        // gml:high "2 1" means width=3, height=2, so 6 values expected. Only 5 provided.
        let xml = build_minimal_valid_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5");
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Mismatch between expected number of elevation values"));
    }
    
    #[test]
    fn test_error_missing_tuple_list() {
         let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "", None, // empty tuple list
            false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true); // omit tuple_list
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Elevation data string (<gml:tupleList>) is missing"));
    }

    // Example of a test for a missing critical element (e.g. gml:high)
    #[test]
    fn test_error_missing_gml_high() {
         let xml = build_test_xml("urn:ogc:def:crs:EPSG::6667", "2 1", "36.0 139.0", "0.1 0.0", "0.0 -0.1", "1 2 3 4 5 6", None,
            false, false, false, false, false, false, true, false, false, false, false, false, false, false, false, false); // omit gml:high
        let result = parse_dem_xml(&xml);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Grid dimensions"));
    }
}

[end of dem_converter/src/xml_parser.rs]
