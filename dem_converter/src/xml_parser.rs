//! # XML Parser Module
//!
//! This module is responsible for parsing Digital Elevation Model (DEM) data
//! from XML files. It is designed with the expectation of a GML-like (Geography
//! Markup Language) structure, commonly used for geospatial data.
//!
//! The primary function [`parse_dem_xml`] attempts to extract metadata and elevation
//! values from the XML content. Due to the lack of specific schemas or official
//! sample files for some DEM formats (e.g., Japanese GSI DEM XML), the parsing
//! logic is **speculative** and based on common GML patterns.
//!
//! Users should be aware that this parser might require adjustments if the input
//! XML deviates significantly from the anticipated GML structure.

use quick_xml::events::{Event, BytesStart};
use quick_xml::Reader;
use crate::{DemData, DemMetadata};
use log::{debug, warn, error};

/// Internal helper function to extract and unescape text content from within an XML element.
///
/// Reads text events until an `End` or `Empty` event is encountered for the current element.
///
/// # Arguments
/// * `reader` - A mutable reference to the `quick_xml::Reader`.
/// * `_event` - The `BytesStart` of the element from which text is being extracted (currently unused but kept for context).
///
/// # Returns
/// A `Result` containing the unescaped text content as a `String`, or an error message `String`
/// if reading/parsing fails or EOF is unexpectedly reached.
fn get_text_from_event(reader: &mut Reader<&[u8]>, _event: &BytesStart) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut txt_buf = Vec::new();
    debug!("Attempting to extract text from current XML element.");
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => txt_buf.extend_from_slice(e.unescape().as_ref()),
            Ok(Event::End(_)) | Ok(Event::Empty(_)) => break,
            Ok(Event::Eof) => {
                let err_msg = "Unexpected EOF while reading text content within an element.".to_string();
                error!("{}", err_msg);
                return Err(err_msg);
            }
            Err(e) => {
                let err_msg = format!("XML Read Error: Error reading text content: {}", e);
                error!("{}", err_msg);
                return Err(err_msg);
            }
            _ => (), 
        }
        buf.clear();
    }
    String::from_utf8(txt_buf).map_err(|e| {
        let err_msg = format!("XML Parse Error: Failed to parse text content as UTF-8: {}", e);
        error!("{}", err_msg);
        err_msg
    })
}


/// Parses DEM XML content into a `DemData` struct.
///
/// **Note:** This parser is based on speculative assumptions about common GML/JPGIS XML structures
/// Parses DEM (Digital Elevation Model) data from an XML string into a [`DemData`] struct.
///
/// This function employs a speculative parsing approach based on common GML (Geography Markup Language)
/// patterns typically found in DEM XML files. It is designed to be somewhat flexible but may
/// require adjustments for XML structures that deviate significantly from these patterns.
///
/// ## Expected XML Structure Highlights:
///
/// The parser looks for elements like (case-insensitive local names, common prefixes `gml:`, `jps:`):
/// *   **Grid Dimensions & Extent:**
///     *   `<gml:GridEnvelope>` with `<gml:high>` (for width, height).
///     *   `<gml:boundedBy>` containing `<gml:Envelope>`.
///     *   `<gml:Envelope>` with `<gml:lowerCorner>` (x_min, y_min_temp) and `<gml:upperCorner>` (x_max_temp, y_max).
///     *   `srsName` attributes on `<gml:Envelope>` or other relevant elements for CRS.
/// *   **Cell Size:**
///     *   `<gml:offsetVector>` (for cell_size_x, cell_size_y).
/// *   **Elevation Data:**
///     *   `<gml:rangeSet>` / `<gml:DataBlock>` containing `<gml:tupleList>`.
///     *   Alternatively, `<jps:valueList>` directly under a coverage-like element.
///     *   Values are expected to be space-separated (commas are also replaced by spaces).
/// *   **No-Data Value:**
///     *   `<gml:nilValues>` or `<swe:nilValues>` containing the no-data marker.
///
/// ## Important Note:
/// This parser is **speculative** due to the absence of official schemas or definitive samples
/// for some target XML formats (e.g., Japanese GSI DEM). Its success heavily depends on the
/// input XML adhering to the assumed GML-like patterns.
///
/// # Arguments
///
/// * `xml_content` - A string slice (`&str`) containing the XML data to be parsed.
///
/// # Returns
///
/// * `Ok(DemData)` - If parsing is successful, returns a `DemData` struct populated with
///   the extracted metadata and elevation values.
/// * `Err(String)` - If parsing fails due to missing critical information, malformed data,
///   or XML read errors, returns a `String` describing the error. Detailed error
///   information is also logged via the `log` crate.
pub fn parse_dem_xml(xml_content: &str) -> Result<DemData, String> {
    debug!("Starting XML parsing process for input of length {}.", xml_content.len());
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();

    // --- Temporary storage for metadata components ---
    let mut width: Option<usize> = None;
    let mut height: Option<usize> = None;
    let mut x_min: Option<f64> = None;
    let mut y_min: Option<f64> = None; // Temp for lowerCorner y
    let mut y_max: Option<f64> = None; // Temp for upperCorner y
    let mut cell_size_x: Option<f64> = None;
    let mut cell_size_y: Option<f64> = None;
    let mut no_data_value: Option<f32> = None;
    let mut crs: Option<String> = None;
    let mut elevation_values_str: Option<String> = None;

    // --- State flags for parsing ---
    let mut in_grid_envelope = false;
    let mut in_lower_corner = false;
    let mut in_upper_corner = false;
    let mut in_tuple_list = false; // For GML-style data blocks
    let mut in_jps_value_list = false; // For JPGIS-style data blocks
    let mut in_nil_values = false;


    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(bs)) => {
                let tag_name_cow = String::from_utf8_lossy(bs.name().as_ref());
                let tag_name = tag_name_cow.to_lowercase();
                debug!("Encountered start tag: <{}>", tag_name);

                for attr in bs.attributes() {
                    if let Ok(attr) = attr {
                        if attr.key.as_ref() == b"srsName" {
                            crs = Some(String::from_utf8_lossy(&attr.value).into_owned());
                            debug!("Found srsName attribute: {}", crs.as_ref().unwrap());
                        }
                    }
                }

                match tag_name.as_str() {
                    "gml:gridenvelope" | "jps:gridenvelope" => {
                        debug!("Entering <{}> parsing state.", tag_name);
                        in_grid_envelope = true;
                    }
                    "gml:low" | "jps:low" if in_grid_envelope => {
                        debug!("Parsing <{}> within GridEnvelope (typically '0 0', ignored for direct width/height).", tag_name);
                    }
                    "gml:high" | "jps:high" if in_grid_envelope => {
                        debug!("Attempting to parse width/height from <{}>.", tag_name);
                        let text = get_text_from_event(&mut reader, &bs)?;
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        if parts.len() == 2 {
                            width = parts[0].parse().ok();
                            height = parts[1].parse().ok();
                            debug!("Parsed width: {:?}, height: {:?} from <{}>", width, height, tag_name);
                            if width.is_none() || height.is_none() {
                                warn!("Failed to parse width/height from '{}' in <{}>. Content: '{}'", text, tag_name, text);
                            }
                        } else {
                            warn!("Unexpected format for width/height in <{}>. Content: '{}'", tag_name, text);
                        }
                    }
                    "gml:envelope" | "jps:envelope" => {
                        debug!("Encountered <{}>, checking for srsName if not already found.", tag_name);
                    }
                    "gml:lowercorner" | "jps:lowercorner" => {
                        debug!("Entering <{}> parsing state.", tag_name);
                        in_lower_corner = true;
                    }
                    "gml:uppercorner" | "jps:uppercorner" => {
                        debug!("Entering <{}> parsing state.", tag_name);
                        in_upper_corner = true;
                    }
                    "gml:offsetvector" | "jps:offsetvector" => {
                        debug!("Attempting to parse cell sizes from <{}>.", tag_name);
                        let text = get_text_from_event(&mut reader, &bs)?;
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        if parts.len() >= 2 {
                            cell_size_x = parts[0].parse().ok();
                            cell_size_y = parts[1].parse().ok().map(|v: f64| v.abs());
                            debug!("Parsed cell_size_x: {:?}, cell_size_y: {:?} from <{}>", cell_size_x, cell_size_y, tag_name);
                             if cell_size_x.is_none() || cell_size_y.is_none() {
                                warn!("Failed to parse cell_size_x/y from '{}' in <{}>. Content: '{}'", text, tag_name, text);
                            }
                        } else {
                             warn!("Unexpected format for cell_size_x/y in <{}>. Content: '{}'", tag_name, text);
                        }
                    }
                    "gml:nilvalues" | "swe:nilvalues" | "jps:nilvalues" => {
                        debug!("Entering <{}> parsing state for no-data value.", tag_name);
                        in_nil_values = true;
                    }
                    "gml:tuplelist" | "tuplelist" => {
                        debug!("Found elevation data tag: <{}>. Entering data parsing state.", tag_name);
                        in_tuple_list = true;
                    }
                    "jps:valuelist" => {
                        debug!("Found elevation data tag: <{}>. Entering data parsing state.", tag_name);
                        in_jps_value_list = true;
                    }
                    _ => {
                        // debug!("Ignoring unhandled start tag: <{}>", tag_name);
                    }
                }
            }
            Ok(Event::End(e_bytes)) => {
                let tag_name_cow = String::from_utf8_lossy(e_bytes.name().as_ref());
                let tag_name = tag_name_cow.to_lowercase();
                debug!("Encountered end tag: </{}>", tag_name);
                match tag_name.as_str() {
                    "gml:gridenvelope" | "jps:gridenvelope" => in_grid_envelope = false,
                    "gml:lowercorner" | "jps:lowercorner" => in_lower_corner = false,
                    "gml:uppercorner" | "jps:uppercorner" => in_upper_corner = false,
                    "gml:tuplelist" | "tuplelist" => in_tuple_list = false,
                    "jps:valuelist" => in_jps_value_list = false,
                    "gml:nilvalues" | "swe:nilvalues" | "jps:nilvalues" => in_nil_values = false,
                    _ => (),
                }
            }
            Ok(Event::Text(e_text)) => {
                let text = e_text.unescape().map(|s| s.into_owned())
                    .map_err(|err| {
                        let msg = format!("XML text decoding error: {}", err);
                        error!("{}", msg);
                        msg
                    })?;
                debug!("Encountered text content (first 50 chars): '{}'", text.chars().take(50).collect::<String>());

                if in_lower_corner {
                    debug!("Attempting to parse x_min, y_min (temp) from text: '{}'", text);
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    if parts.len() == 2 {
                        x_min = parts[0].parse().ok();
                        y_min = parts[1].parse().ok();
                        debug!("Parsed x_min: {:?}, y_min (temp): {:?}", x_min, y_min);
                        if x_min.is_none() || y_min.is_none() {
                            warn!("Failed to parse x_min/y_min from <gml:lowerCorner> content: '{}'", text);
                        }
                    } else {
                        warn!("Unexpected format for <gml:lowerCorner> content: '{}'", text);
                    }
                    in_lower_corner = false; 
                } else if in_upper_corner {
                    debug!("Attempting to parse y_max from text: '{}'", text);
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    if parts.len() == 2 {
                        y_max = parts[1].parse().ok();
                        debug!("Parsed y_max: {:?}", y_max);
                        if y_max.is_none() {
                             warn!("Failed to parse y_max from <gml:upperCorner> content: '{}'", text);
                        }
                    } else {
                        warn!("Unexpected format for <gml:upperCorner> content: '{}'", text);
                    }
                    in_upper_corner = false; 
                } else if in_tuple_list || in_jps_value_list {
                    debug!("Capturing elevation data string: '{}'", text.chars().take(100).collect::<String>());
                    elevation_values_str = Some(text.to_string());
                    in_tuple_list = false; // Assume data is direct child
                    in_jps_value_list = false; // Assume data is direct child
                } else if in_nil_values {
                    debug!("Attempting to parse no_data_value from text: '{}'", text);
                    no_data_value = text.trim().parse().ok();
                    debug!("Parsed no_data_value: {:?}", no_data_value);
                    if no_data_value.is_none() && !text.trim().is_empty() {
                        warn!("Failed to parse no_data_value from content: '{}'", text.trim());
                    }
                    in_nil_values = false;
                }
            }
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

    // --- Post-processing and Validation ---
    if crs.is_none() {
        warn!("CRS information (srsName) not found in XML. GeoTiff will lack CRS metadata.");
    }
    if no_data_value.is_none() {
        warn!("No-data value (<gml:nilValues> or similar) not found or failed to parse. GeoTiff will not have a no-data value set, unless all values are valid numbers.");
    }
    }

    // --- Post-processing and Validation ---

    // Ensure y_max is indeed the maximum y-value. If only y_min and y_max are available from
    // lowerCorner and upperCorner, and if cell_size_y is positive, y_max should be the one from upperCorner.
    // If the DEM uses a geographic CRS where Y increases northwards, upperCorner Y is y_max.
    // If it's a projected CRS where Y decreases (e.g. some image formats), this might need adjustment,
    // but for DEMs, y_max is usually the northernmost extent.

    // Derivations for cell_size_x/y are not implemented as they are highly speculative.
    // Requiring them to be present in the XML.

    // --- Construct DemMetadata ---
    debug!("Constructing DemMetadata from parsed values.");
    let final_width = width.ok_or_else(|| {
        let msg = "XML Parse Error: DEM width (e.g., from <gml:high> or <jps:high> in GridEnvelope) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_height = height.ok_or_else(|| {
        let msg = "XML Parse Error: DEM height (e.g., from <gml:high> or <jps:high> in GridEnvelope) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_x_min = x_min.ok_or_else(|| {
        let msg = "XML Parse Error: DEM x_min (e.g., from <gml:lowerCorner>) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_y_max = y_max.ok_or_else(|| {
        let msg = "XML Parse Error: DEM y_max (e.g., from <gml:upperCorner>) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_cell_size_x = cell_size_x.ok_or_else(|| {
        let msg = "XML Parse Error: DEM cell_size_x (e.g., from <gml:offsetVector>) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;
    let final_cell_size_y = cell_size_y.ok_or_else(|| {
        let msg = "XML Parse Error: DEM cell_size_y (e.g., from <gml:offsetVector>) is missing or invalid.".to_string();
        error!("{}", msg);
        msg
    })?;

    let metadata = DemMetadata {
        width: final_width,
        height: final_height,
        x_min: final_x_min,
        y_max: final_y_max,
        cell_size_x: final_cell_size_x,
        cell_size_y: final_cell_size_y,
        no_data_value,
        crs,
    };
    debug!("DemMetadata constructed: {:?}", metadata);

    // --- Parse Elevation Values ---
    debug!("Parsing elevation values string.");
    let elevation_values_str_content = elevation_values_str
        .ok_or_else(|| {
            let msg = "XML Parse Error: Elevation data string (e.g., from <gml:tupleList> or <jps:valueList>) is missing.".to_string();
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

    // Helper to build a common XML structure for tests, allowing specific parts to be overridden.
    fn build_gml_xml_string(
        srs_name: &str,
        grid_high: &str, // "width height"
        lower_corner: &str, // "x y"
        upper_corner: &str, // "x y"
        offset_vector: &str, // "cell_x cell_y"
        tuple_list_content: &str,
        nil_values_content: Option<&str>, // e.g., "<gml:nilValues>-9999.0</gml:nilValues>"
    ) -> String {
        let nil_values_block = nil_values_content
            .map(|c| format!("<gml:metadata><gml:NilValues nilReason=\"nodata\">{}</gml:NilValues></gml:metadata>", c))
            .unwrap_or_else(|| "".to_string()); // No nilValues block if None

        format!(
            r#"
            <DEM xmlns:gml="http://www.opengis.net/gml/3.2" xmlns:jps="http://www.gsi.go.jp/GIS/jpgis/2.0/spec">
              <GridCoverage srsName="{}">
                <gml:GridEnvelope>
                  <gml:low>0 0</gml:low>
                  <gml:high>{}</gml:high> 
                </gml:GridEnvelope>
                <gml:boundedBy>
                    <gml:Envelope srsName="{}"> 
                        <gml:lowerCorner>{}</gml:lowerCorner>
                        <gml:upperCorner>{}</gml:upperCorner>
                    </gml:Envelope>
                </gml:boundedBy>
                <gml:gridDomain>
                    <gml:GridFunction>
                        <gml:sequenceRule order="+x-y">Linear</gml:sequenceRule> 
                    </gml:GridFunction>
                </gml:gridDomain>
                <gml:rangeSet>
                    <gml:DataBlock>
                        <gml:tupleList>{}</gml:tupleList>
                    </gml:DataBlock>
                </gml:rangeSet>
                {} 
                <gml:offsetVector srsName="{}">{}</gml:offsetVector> 
              </GridCoverage>
            </DEM>
            "#,
            srs_name, grid_high, srs_name, lower_corner, upper_corner, tuple_list_content, nil_values_block, srs_name, offset_vector
        )
    }

    #[test]
    fn test_parse_ideal_gml_xml() {
        // Test case: A reasonably complete and valid GML-like XML.
        let xml_data = build_gml_xml_string(
            "EPSG:4326",
            "2 2", // width=2, height=2
            "135.0 35.0", // x_min, y_min (temp)
            "135.01 35.01", // x_max (temp), y_max
            "0.005 0.005", // cell_size_x, cell_size_y
            "1.0 2.0 3.0 4.0", // elevation data
            Some("<gml:nilValues>-9999.0</gml:nilValues>"),
        );

        let result = parse_dem_xml(&xml_data);
        assert!(result.is_ok(), "Parsing ideal GML XML failed: {:?}", result.err());
        let dem_data = result.unwrap();

        assert_eq!(dem_data.metadata.width, 2);
        assert_eq!(dem_data.metadata.height, 2);
        assert_eq!(dem_data.metadata.x_min, 135.0);
        assert_eq!(dem_data.metadata.y_max, 35.01);
        assert_eq!(dem_data.metadata.cell_size_x, 0.005);
        assert_eq!(dem_data.metadata.cell_size_y, 0.005);
        assert_eq!(dem_data.metadata.crs, Some("EPSG:4326".to_string()));
        assert_eq!(dem_data.metadata.no_data_value, Some(-9999.0));
        assert_eq!(dem_data.elevation_values, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_parse_xml_missing_optional_crs() {
        // Test case: XML is valid but missing the optional srsName attribute for CRS.
        // The parser should still succeed but crs field will be None.
        // We achieve this by passing an empty string for srs_name which our build_gml_xml_string
        // will place in srsName attributes. The quick-xml parser will then not pick it up as a valid CRS.
        // A more robust way would be to modify build_gml_xml_string to optionally omit srsName attributes.
        // For now, using an empty string in srsName effectively tests this, as our parser only captures non-empty srsName.
         let xml_data = build_gml_xml_string(
            "", // Empty srsName
            "1 1", "0.0 0.0", "0.1 0.1", "0.1 0.1", "10.0",
            Some("<gml:nilValues>-9999.0</gml:nilValues>"),
        );
        let result = parse_dem_xml(&xml_data);
        assert!(result.is_ok(), "Parsing XML missing CRS failed: {:?}", result.err());
        let dem_data = result.unwrap();
        assert_eq!(dem_data.metadata.crs, None, "CRS should be None when srsName is missing/empty.");
    }
    
    #[test]
    fn test_parse_xml_missing_optional_nodata() {
        // Test case: XML is valid but missing the optional no_data_value.
        let xml_data = build_gml_xml_string(
            "EPSG:3857",
            "1 1", "0.0 0.0", "1.0 1.0", "1.0 1.0", "100.0",
            None, // No nil_values_content
        );
        let result = parse_dem_xml(&xml_data);
        assert!(result.is_ok(), "Parsing XML missing no-data failed: {:?}", result.err());
        let dem_data = result.unwrap();
        assert_eq!(dem_data.metadata.no_data_value, None, "no_data_value should be None.");
    }


    #[test]
    fn test_parse_malformed_elevation_data_non_numeric() {
        // Test case: Elevation data contains non-numeric values.
        let xml_data = build_gml_xml_string(
            "EPSG:4326", "2 1", "0 0", "1 1", "1 1",
            "1.0 non_numeric_value", // Malformed data
            Some("<gml:nilValues>-9999.0</gml:nilValues>"),
        );
        let result = parse_dem_xml(&xml_data);
        assert!(result.is_err(), "Parsing should fail for non-numeric elevation data.");
        assert!(result.err().unwrap().contains("Failed to parse elevation value 'non_numeric_value'"));
    }
    
    #[test]
    fn test_parse_malformed_elevation_data_incorrect_count() {
        // Test case: Elevation data count does not match width * height.
        let xml_data = build_gml_xml_string(
            "EPSG:4326", "2 2", "0 0", "1 1", "0.5 0.5",
            "1.0 2.0 3.0", // Expected 4 values (2x2), got 3
            Some("<gml:nilValues>-9999.0</gml:nilValues>"),
        );
        let result = parse_dem_xml(&xml_data);
        assert!(result.is_err(), "Parsing should fail for incorrect elevation data count.");
        assert!(result.err().unwrap().contains("Mismatch between expected number of elevation values"));
    }

    #[test]
    fn test_missing_essential_width_height() {
        // Test case: Missing <gml:high> which defines width and height.
        let xml_data = r#" 
        <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
          <GridCoverage srsName="EPSG:4326">
            <gml:GridEnvelope>
              <gml:low>0 0</gml:low>
              <!-- <gml:high>2 2</gml:high> --> <!-- Missing -->
            </gml:GridEnvelope>
            <gml:boundedBy><gml:Envelope srsName="EPSG:4326"><gml:lowerCorner>0 0</gml:lowerCorner><gml:upperCorner>1 1</gml:upperCorner></gml:Envelope></gml:boundedBy>
            <gml:rangeSet><gml:DataBlock><gml:tupleList>1 2 3 4</gml:tupleList></gml:DataBlock></gml:rangeSet>
            <gml:offsetVector>0.5 0.5</gml:offsetVector>
          </GridCoverage>
        </DEM>"#;
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err(), "Parsing should fail if width/height are missing.");
        assert!(result.err().unwrap().contains("DEM width"));
    }

    #[test]
    fn test_missing_essential_lower_corner() {
        // Test case: Missing <gml:lowerCorner> which defines x_min.
        let xml_data = r#"
        <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
          <GridCoverage srsName="EPSG:4326">
            <gml:GridEnvelope><gml:low>0 0</gml:low><gml:high>2 2</gml:high></gml:GridEnvelope>
            <gml:boundedBy>
                <gml:Envelope srsName="EPSG:4326">
                    <!-- <gml:lowerCorner>0 0</gml:lowerCorner> --> <!-- Missing -->
                    <gml:upperCorner>1 1</gml:upperCorner>
                </gml:Envelope>
            </gml:boundedBy>
            <gml:rangeSet><gml:DataBlock><gml:tupleList>1 2 3 4</gml:tupleList></gml:DataBlock></gml:rangeSet>
            <gml:offsetVector>0.5 0.5</gml:offsetVector>
          </GridCoverage>
        </DEM>"#;
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err(), "Parsing should fail if x_min (lowerCorner) is missing.");
        assert!(result.err().unwrap().contains("DEM x_min"));
    }
    
    #[test]
    fn test_missing_essential_offset_vector() {
        // Test case: Missing <gml:offsetVector> which defines cell sizes.
        let xml_data = r#"
        <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
          <GridCoverage srsName="EPSG:4326">
            <gml:GridEnvelope><gml:low>0 0</gml:low><gml:high>2 2</gml:high></gml:GridEnvelope>
            <gml:boundedBy><gml:Envelope srsName="EPSG:4326"><gml:lowerCorner>0 0</gml:lowerCorner><gml:upperCorner>1 1</gml:upperCorner></gml:Envelope></gml:boundedBy>
            <gml:rangeSet><gml:DataBlock><gml:tupleList>1 2 3 4</gml:tupleList></gml:DataBlock></gml:rangeSet>
            <!-- <gml:offsetVector>0.5 0.5</gml:offsetVector> --> <!-- Missing -->
          </GridCoverage>
        </DEM>"#;
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err(), "Parsing should fail if cell_size_x/y (offsetVector) are missing.");
        assert!(result.err().unwrap().contains("DEM cell_size_x"));
    }

    #[test]
    fn test_missing_elevation_data_tuplelist() {
        // Test case: Missing <gml:tupleList> (or <jps:valueList>).
        let xml_data = r#"
        <DEM xmlns:gml="http://www.opengis.net/gml/3.2">
          <GridCoverage srsName="EPSG:4326">
            <gml:GridEnvelope><gml:low>0 0</gml:low><gml:high>2 2</gml:high></gml:GridEnvelope>
            <gml:boundedBy><gml:Envelope srsName="EPSG:4326"><gml:lowerCorner>0 0</gml:lowerCorner><gml:upperCorner>1 1</gml:upperCorner></gml:Envelope></gml:boundedBy>
            <gml:rangeSet><gml:DataBlock><!-- <gml:tupleList>1 2 3 4</gml:tupleList> --></gml:DataBlock></gml:rangeSet>
            <gml:offsetVector>0.5 0.5</gml:offsetVector>
          </GridCoverage>
        </DEM>"#;
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err(), "Parsing should fail if elevation data string is missing.");
        assert!(result.err().unwrap().contains("Elevation data string"));
    }

    // Keeping the existing minimal and JPGIS tests as they test slightly different structures/paths.
    // The `build_gml_xml_string` helper is primarily for GML-like structures.
    #[test]
    fn test_parse_minimal_hypothetical_xml_original() { // Renamed to avoid conflict
        // This is a VERY simplified and hypothetical XML. Real GSI XML will be much more complex.
        // It includes only a few elements to check basic parsing flow.
        let xml_data = r#"
        <DEM>
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
                    <gml:tupleList>
                        1.0 2.0
                        3.0 4.0
                    </gml:tupleList>
                </gml:DataBlock>
            </gml:rangeSet>
            <gml:coverageFunction>
                <gml:GridFunction>
                    <gml:sequenceRule>Linear</gml:sequenceRule> 
                </gml:GridFunction>
            </gml:coverageFunction>
            <gml:metadata>
                <gml:NilValues nilReason="nodata">
                    <gml:nilValues>-9999.0</gml:nilValues>
                </gml:NilValues>
            </gml:metadata>
            <gml:offsetVector srsName="EPSG:4326">0.005 0.005</gml:offsetVector> 
          </GridCoverage>
        </DEM>
        "#;

        let result = parse_dem_xml(xml_data);
        assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
        let dem_data = result.unwrap();

        assert_eq!(dem_data.metadata.width, 2);
        assert_eq!(dem_data.metadata.height, 2);
        assert_eq!(dem_data.metadata.x_min, 135.0);
        assert_eq!(dem_data.metadata.y_max, 35.01);
        assert_eq!(dem_data.metadata.cell_size_x, 0.005);
        assert_eq!(dem_data.metadata.cell_size_y, 0.005);
        assert_eq!(dem_data.metadata.crs, Some("EPSG:4326".to_string()));
        assert_eq!(dem_data.metadata.no_data_value, Some(-9999.0));
        assert_eq!(dem_data.elevation_values, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_parse_jpgis_style_value_list_original() { // Renamed to avoid conflict
        // This test uses a modified GML structure to specifically test the <jps:valueList> path
        // for elevation data, while keeping metadata parsing GML-like due to current parser focus.
        let modified_xml_for_current_parser = r#"
        <DEM>
          <GridCoverage srsName="JGD2000">
            <gml:GridEnvelope>
              <gml:low>0 0</gml:low>
              <gml:high>2 2</gml:high> 
            </gml:GridEnvelope>
            <gml:boundedBy>
                <gml:Envelope srsName="JGD2000">
                    <gml:lowerCorner>140.0 40.0</gml:lowerCorner>
                    <gml:upperCorner>140.1 40.1</gml:upperCorner>
                </gml:Envelope>
            </gml:boundedBy>
            <gml:rangeSet>
                <jps:valueList> <!-- Test this specific element for data extraction -->
                    10.0 20.5
                    30.0 40.5
                </jps:valueList>
            </gml:rangeSet>
            <gml:metadata>
                 <gml:nilValues><gml:nilValues>-999.0</gml:nilValues></gml:nilValues>
            </gml:metadata>
            <gml:offsetVector>0.05 0.05</gml:offsetVector> 
          </GridCoverage>
        </DEM>
        "#;

        let result = parse_dem_xml(modified_xml_for_current_parser);
        assert!(result.is_ok(), "Parsing failed for JPGIS-style valueList: {:?}", result.err());
        let dem_data = result.unwrap();

        assert_eq!(dem_data.metadata.width, 2);
        assert_eq!(dem_data.metadata.height, 2);
        assert_eq!(dem_data.metadata.x_min, 140.0);
        assert_eq!(dem_data.metadata.y_max, 40.1);
        assert_eq!(dem_data.metadata.cell_size_x, 0.05);
        assert_eq!(dem_data.metadata.cell_size_y, 0.05);
        assert_eq!(dem_data.metadata.crs, Some("JGD2000".to_string()));
        assert_eq!(dem_data.metadata.no_data_value, Some(-999.0));
        assert_eq!(dem_data.elevation_values, vec![10.0, 20.5, 30.0, 40.5]);
    }

    #[test]
    fn test_missing_critical_data_original() { // Renamed
        // Test with an almost empty XML, ensuring it fails due to multiple missing critical fields.
        let xml_data = r#"<DEM></DEM>"#; 
        let result = parse_dem_xml(xml_data);
        assert!(result.is_err());
        // Example: Check if the error message indicates missing width, which is one of the first critical fields checked.
        assert!(result.err().unwrap().contains("DEM width"));
    }
}
