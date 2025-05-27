use std::io::{BufRead, Cursor};
use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::model::{DemTile, Metadata};

pub fn parse_dem_xml<R: BufRead>(reader: R) -> Result<DemTile> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut meshcode = None;
    let mut dem_type = None;
    let mut crs_identifier = None;
    let mut grid_high = None;
    let mut lower_corner = None;
    let mut upper_corner = None;
    let mut start_point = None;
    let mut values = Vec::new();

    let mut in_mesh = false;
    let mut in_type = false;
    let mut in_envelope = false;
    let mut in_lower_corner = false;
    let mut in_upper_corner = false;
    let mut in_grid_envelope = false;
    let mut in_high = false;
    let mut in_tuple_list = false;
    let mut in_start_point = false;

    let mut buf = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.local_name().as_ref() {
                    b"mesh" => in_mesh = true,
                    b"type" => in_type = true,
                    b"Envelope" => {
                        in_envelope = true;
                        // srsName属性から座標系を取得
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"srsName" {
                                crs_identifier = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                        }
                    }
                    b"lowerCorner" => in_lower_corner = true,
                    b"upperCorner" => in_upper_corner = true,
                    b"GridEnvelope" => in_grid_envelope = true,
                    b"high" => if in_grid_envelope { in_high = true; }
                    b"tupleList" => in_tuple_list = true,
                    b"startPoint" => in_start_point = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_mesh {
                    meshcode = Some(e.unescape()?.to_string());
                } else if in_type {
                    dem_type = Some(e.unescape()?.to_string());
                } else if in_lower_corner && in_envelope {
                    lower_corner = Some(e.unescape()?.to_string());
                } else if in_upper_corner && in_envelope {
                    upper_corner = Some(e.unescape()?.to_string());
                } else if in_high && in_grid_envelope {
                    grid_high = Some(e.unescape()?.to_string());
                } else if in_start_point {
                    start_point = Some(e.unescape()?.to_string());
                } else if in_tuple_list {
                    // tupleListのテキストをストリーム処理
                    let text = e.unescape()?;
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        // カンマで分割し、2番目の要素（標高値）を取得
                        let parts: Vec<&str> = line.split(',').collect();
                        if parts.len() >= 2 {
                            let elevation = parts[1].trim().parse::<f32>()
                                .with_context(|| format!("Failed to parse elevation: {}", parts[1]))?;
                            values.push(elevation);
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                match e.local_name().as_ref() {
                    b"mesh" => in_mesh = false,
                    b"type" => in_type = false,
                    b"Envelope" => in_envelope = false,
                    b"lowerCorner" => in_lower_corner = false,
                    b"upperCorner" => in_upper_corner = false,
                    b"GridEnvelope" => in_grid_envelope = false,
                    b"high" => in_high = false,
                    b"tupleList" => in_tuple_list = false,
                    b"startPoint" => in_start_point = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    // 必須フィールドの検証
    let meshcode = meshcode.context("meshcode not found")?;
    let dem_type = dem_type.context("dem_type not found")?;
    let crs_identifier = crs_identifier.context("crs_identifier not found")?;
    let grid_high = grid_high.context("grid_high not found")?;
    let lower_corner = lower_corner.context("lower_corner not found")?;
    let upper_corner = upper_corner.context("upper_corner not found")?;
    let start_point = start_point.unwrap_or_else(|| "0 0".to_string());

    // グリッドサイズを解析（high値 + 1）
    let high_parts: Vec<&str> = grid_high.split_whitespace().collect();
    if high_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid grid high format: {}", grid_high));
    }
    let cols = high_parts[0].parse::<usize>()? + 1;
    let rows = high_parts[1].parse::<usize>()? + 1;

    // 座標を解析
    let lower_parts: Vec<&str> = lower_corner.split_whitespace().collect();
    let upper_parts: Vec<&str> = upper_corner.split_whitespace().collect();
    if lower_parts.len() != 2 || upper_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid corner coordinate format"));
    }

    // JGD2011 (fguuid:jgd2011.bl) uses lat,lon order
    let origin_lat = lower_parts[0].parse::<f64>()?;
    let origin_lon = lower_parts[1].parse::<f64>()?;
    let upper_lat = upper_parts[0].parse::<f64>()?;
    let upper_lon = upper_parts[1].parse::<f64>()?;

    // 解像度を計算
    let x_res = if cols > 1 {
        (upper_lon - origin_lon) / (cols - 1) as f64
    } else {
        upper_lon - origin_lon
    };
    let y_res = if rows > 1 {
        (upper_lat - origin_lat) / (rows - 1) as f64
    } else {
        upper_lat - origin_lat
    };

    // startPointを解析
    let start_parts: Vec<&str> = start_point.split_whitespace().collect();
    let start_x = if start_parts.len() >= 1 { start_parts[0].parse::<usize>()? } else { 0 };
    let start_y = if start_parts.len() >= 2 { start_parts[1].parse::<usize>()? } else { 0 };

    // startPointを考慮した実際のデータ数を計算
    // startPoint(1056, 0)は最初の1056列がデータ無しを意味する
    let expected_values = rows * cols - start_x;

    // 値の数を検証
    if values.len() != expected_values {
        return Err(anyhow::anyhow!(
            "Value count mismatch: expected {} ({}x{} - start_x {}), got {}",
            expected_values, rows, cols, start_x, values.len()
        ));
    }

    // startPointの分だけ-9999（NoData）を先頭に追加
    let mut full_values = vec![-9999.0; start_x];
    full_values.extend(values);
    let values = full_values;

    let metadata = Metadata {
        meshcode,
        dem_type,
        crs_identifier,
    };

    Ok(DemTile {
        rows,
        cols,
        origin_lon,
        origin_lat: upper_lat,  // DEMの原点は左上なので上端の緯度を使用
        x_res,
        y_res,
        values,
        start_point: (start_x, start_y),
        metadata,
    })
}

pub fn parse_dem_xml_from_bytes(bytes: &[u8]) -> Result<DemTile> {
    let cursor = Cursor::new(bytes);
    parse_dem_xml(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dem() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Dataset xmlns="http://fgd.japan.go.jp/spec/2008/FGD_GMLSchema" xmlns:gml="http://www.opengis.net/gml/3.2">
<DEM>
    <mesh>12345678</mesh>
    <type>1mメッシュ（標高）</type>
    <coverage>
        <gml:boundedBy>
            <gml:Envelope srsName="fguuid:jgd2011.bl">
                <gml:lowerCorner>35.0 135.0</gml:lowerCorner>
                <gml:upperCorner>35.001 135.001</gml:upperCorner>
            </gml:Envelope>
        </gml:boundedBy>
        <gml:gridDomain>
            <gml:Grid>
                <gml:limits>
                    <gml:GridEnvelope>
                        <gml:high>1 1</gml:high>
                    </gml:GridEnvelope>
                </gml:limits>
            </gml:Grid>
        </gml:gridDomain>
        <gml:rangeSet>
            <gml:DataBlock>
                <gml:tupleList>
地表面,100.0
地表面,101.0
地表面,102.0
地表面,103.0
                </gml:tupleList>
            </gml:DataBlock>
        </gml:rangeSet>
        <gml:coverageFunction>
            <gml:GridFunction>
                <gml:startPoint>0 0</gml:startPoint>
            </gml:GridFunction>
        </gml:coverageFunction>
    </coverage>
</DEM>
</Dataset>"#;

        let result = parse_dem_xml(xml.as_bytes()).unwrap();
        assert_eq!(result.rows, 2);
        assert_eq!(result.cols, 2);
        assert_eq!(result.values.len(), 4);
        assert_eq!(result.values[0], 100.0);
        assert_eq!(result.metadata.meshcode, "12345678");
    }
}
