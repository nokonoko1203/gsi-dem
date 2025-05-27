use crate::model::DemTile;
use crate::parser;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use zip::ZipArchive;

pub struct ZipHandler {
    path: PathBuf,
}

impl ZipHandler {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn validate_filename(&self) -> Result<()> {
        let filename = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid filename")?;

        if !filename.starts_with("FG-GML-") || !filename.ends_with(".zip") {
            anyhow::bail!("Invalid ZIP filename format. Expected: FG-GML-****-DEM*A.zip");
        }

        let parts: Vec<&str> = filename.split('-').collect();
        if parts.len() < 4 {
            anyhow::bail!("Invalid filename format");
        }

        if !parts[3].starts_with("DEM") || !parts[3].ends_with("A") {
            anyhow::bail!("Invalid DEM type in filename");
        }

        info!("ZIP file validated: {}", filename);
        Ok(())
    }

    pub fn extract_xml_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let file = File::open(&self.path).context("Failed to open ZIP file")?;
        let mut archive =
            ZipArchive::new(BufReader::new(file)).context("Failed to read ZIP archive")?;

        let mut xml_files = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .context("Failed to access file in ZIP")?;

            let name = file.name().to_string();

            if name.ends_with(".xml") && !name.contains("__MACOSX") {
                debug!("Found XML file: {}", name);

                let mut contents = Vec::new();
                file.read_to_end(&mut contents)
                    .context("Failed to read XML file from ZIP")?;

                xml_files.push((name, contents));
            }
        }

        if xml_files.is_empty() {
            anyhow::bail!("No XML files found in ZIP archive");
        }

        info!("Found {} XML files in ZIP archive", xml_files.len());
        Ok(xml_files)
    }

    pub fn process_all_tiles(&self) -> Result<Vec<DemTile>> {
        self.validate_filename()?;
        let xml_files = self.extract_xml_files()?;

        let mut tiles = Vec::new();

        for (name, contents) in xml_files {
            info!("Processing XML file: {}", name);

            match parser::parse_dem_xml_from_bytes(&contents) {
                Ok(tile) => {
                    debug!(
                        "Successfully parsed tile with mesh code: {:?}",
                        tile.metadata.meshcode
                    );
                    tiles.push(tile);
                }
                Err(e) => {
                    warn!("Failed to parse XML file {}: {}", name, e);
                }
            }
        }

        if tiles.is_empty() {
            anyhow::bail!("Failed to parse any XML files from ZIP");
        }

        info!("Successfully processed {} tiles", tiles.len());
        Ok(tiles)
    }
}

#[derive(Debug, Clone)]
pub struct MergedDemTile {
    pub tiles: Vec<DemTile>,
    pub merged_rows: usize,
    pub merged_cols: usize,
    pub merged_origin_lon: f64,
    pub merged_origin_lat: f64,
    pub merged_x_res: f64,
    pub merged_y_res: f64,
    pub merged_values: Vec<f32>,
    pub crs_identifier: String,
}

impl MergedDemTile {
    pub fn from_tiles(mut tiles: Vec<DemTile>) -> Result<Self> {
        if tiles.is_empty() {
            anyhow::bail!("Cannot merge empty tile list");
        }

        // タイルをメッシュコードでソート（西から東、南から北の順）
        tiles.sort_by(|a, b| a.metadata.meshcode.cmp(&b.metadata.meshcode));

        let first_tile = &tiles[0];
        let crs = first_tile.metadata.crs_identifier.clone();

        for tile in &tiles[1..] {
            let tile_crs = &tile.metadata.crs_identifier;
            if tile_crs != &crs {
                anyhow::bail!("CRS mismatch: {} vs {}", crs, tile_crs);
            }
        }

        let min_lon = tiles
            .iter()
            .map(|t| {
                if t.start_point.0 > 0 {
                    // start_pointがある場合、実際のデータ開始位置
                    t.origin_lon + (t.start_point.0 as f64 * t.x_res)
                } else {
                    t.origin_lon
                }
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_lat = tiles
            .iter()
            .map(|t| t.origin_lat)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        // 実際のデータ範囲を考慮した計算（隣接タイルは1ピクセル重複）
        let max_lon = tiles
            .iter()
            .map(|t| {
                // start_pointがある場合は、実際のデータ終了位置を計算
                let actual_end_col = if t.start_point.0 > 0 {
                    t.cols - 1 // 最後の列まで
                } else {
                    t.cols - 1 // 通常のタイルも最後の列まで
                };
                t.origin_lon + (actual_end_col as f64 * t.x_res)
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let min_lat = tiles
            .iter()
            .map(|t| t.origin_lat - ((t.rows - 1) as f64 * t.y_res.abs()))
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let x_res = first_tile.x_res;
        let y_res = first_tile.y_res;

        // 結合後のサイズを計算
        let merged_cols = ((max_lon - min_lon) / x_res).round() as usize + 1;
        let merged_rows = ((max_lat - min_lat) / y_res.abs()).round() as usize + 1;

        debug!(
            "Merging tiles: min_lon={}, max_lon={}, min_lat={}, max_lat={}",
            min_lon, max_lon, min_lat, max_lat
        );
        debug!("Resolution: x_res={}, y_res={}", x_res, y_res);
        debug!("Merged size: {}x{}", merged_rows, merged_cols);

        let mut merged_values = vec![-9999.0; merged_rows * merged_cols];

        // タイルをグリッド位置でグループ化
        let mut tile_grid: std::collections::HashMap<(i32, i32), &DemTile> =
            std::collections::HashMap::new();
        let mut min_grid_x = i32::MAX;
        let mut min_grid_y = i32::MAX;

        for tile in &tiles {
            // メッシュコードから位置を推定（下2桁がYX）
            let code = &tile.metadata.meshcode;
            if code.len() >= 8 {
                let last_two = &code[code.len() - 2..];
                let two_digit_num = last_two.parse::<u32>().unwrap();
                let grid_y = (two_digit_num / 10) as i32; // 十の位がY
                let grid_x = (two_digit_num % 10) as i32; // 一の位がX
                tile_grid.insert((grid_x, grid_y), tile);
                min_grid_x = min_grid_x.min(grid_x);
                min_grid_y = min_grid_y.min(grid_y);
            }
        }

        for (i, tile) in tiles.iter().enumerate() {
            // グリッド位置を取得（下2桁がYX）
            let code = &tile.metadata.meshcode;
            let (grid_x, grid_y) = if code.len() >= 8 {
                let last_two = &code[code.len() - 2..];
                let two_digit_num = last_two.parse::<u32>().unwrap_or(0);
                let y = (two_digit_num / 10) as i32; // 十の位がY
                let x = (two_digit_num % 10) as i32; // 一の位がX
                (x, y)
            } else {
                (0, 0)
            };

            // タイルの左端位置を計算（start_pointを考慮）
            let actual_origin_lon = if tile.start_point.0 > 0 {
                // start_pointがある場合、実際のデータは右側にずれている
                tile.origin_lon + (tile.start_point.0 as f64 * tile.x_res)
            } else {
                tile.origin_lon
            };

            // グリッド位置からオフセットを計算（重複を考慮）
            // 実際のタイルサイズを使用
            let tile_width_with_overlap = tile.cols;
            let tile_height_with_overlap = tile.rows;
            let overlap = 1;
            let effective_tile_width = tile_width_with_overlap - overlap;
            let effective_tile_height = tile_height_with_overlap - overlap;

            // グリッド位置に基づくオフセット計算（重複を考慮）
            let grid_offset_x = (grid_x - min_grid_x) as usize;
            let grid_offset_y = (grid_y - min_grid_y) as usize;

            // メッシュコードのY座標は北が大きい（Y=9が最北端）
            // 画像の行インデックスは上から下へ増加する
            // したがって、Y座標が大きいタイルほど行インデックスは小さくなる
            let max_grid_y = 9i32; // 10x10グリッドの最大Y座標
            let inverted_grid_y = (max_grid_y - grid_y) as usize;
            let tile_col_offset = grid_offset_x * effective_tile_width;
            let tile_row_offset = inverted_grid_y * effective_tile_height;

            debug!("Tile {}: meshcode={}, grid=({},{}), inverted_y={}, origin=({}, {}), actual_origin_lon={}, offset=({}, {}), size={}x{}, start_point={:?}",
                i,
                tile.metadata.meshcode,
                grid_x,
                grid_y,
                inverted_grid_y,
                tile.origin_lon,
                tile.origin_lat,
                actual_origin_lon,
                tile_col_offset,
                tile_row_offset,
                tile.rows,
                tile.cols,
                tile.start_point
            );

            // グリッド位置に基づいて、左端/上端かどうかを判定
            let is_leftmost_in_grid = grid_x == min_grid_x || tile.start_point.0 > 0;
            let is_topmost_in_grid = grid_y == min_grid_y;

            if i < 20 {
                // 最初の20タイルでデバッグ
                debug!(
                    "Tile {} grid=({},{}), is_leftmost={}, is_topmost={}, offset=({},{})",
                    i,
                    grid_x,
                    grid_y,
                    is_leftmost_in_grid,
                    is_topmost_in_grid,
                    tile_col_offset,
                    tile_row_offset
                );
            }

            // データコピー（重複を考慮）
            for row in 0..tile.rows {
                for col in 0..tile.cols {
                    // 重複ピクセルのスキップ判定
                    // グリッド内で左端でない場合、左端の列をスキップ
                    if grid_x > min_grid_x && col == 0 {
                        continue;
                    }
                    // グリッド内で上端でない場合、上端の行をスキップ
                    // Y座標が大きいほど北なので、grid_y < max_grid_yの場合は上端ではない
                    if grid_y < 9 && row == 0 {
                        continue;
                    }

                    let src_idx = row * tile.cols + col;
                    if src_idx < tile.values.len() {
                        let value = tile.values[src_idx];

                        // NoDataでない値のみを書き込む
                        if value != -9999.0 {
                            // 調整後の位置を計算（スキップした分を考慮）
                            let adjusted_row = if grid_y < 9 { row - 1 } else { row };
                            let adjusted_col = if grid_x > min_grid_x { col - 1 } else { col };

                            let dst_row = tile_row_offset + adjusted_row;
                            let dst_col = tile_col_offset + adjusted_col;

                            if dst_row < merged_rows && dst_col < merged_cols {
                                let dst_idx = dst_row * merged_cols + dst_col;

                                // デバッグ: 境界部分のデータ
                                if i < 11 && (row < 2 || col < 2) {
                                    debug!("Tile {} (grid {},{}) data[{},{}] = {} -> merged[{},{}] (skip_top={}, skip_left={})",
                                        i, grid_x, grid_y, row, col, value, dst_row, dst_col,
                                        grid_y < 9 && row == 0, grid_x > min_grid_x && col == 0);
                                }

                                merged_values[dst_idx] = value;
                            }
                        }
                    }
                }
            }
        }

        Ok(MergedDemTile {
            tiles,
            merged_rows,
            merged_cols,
            merged_origin_lon: min_lon,
            merged_origin_lat: max_lat,
            merged_x_res: x_res,
            merged_y_res: y_res,
            merged_values,
            crs_identifier: crs,
        })
    }

    pub fn to_dem_tile(&self) -> DemTile {
        DemTile {
            rows: self.merged_rows,
            cols: self.merged_cols,
            origin_lon: self.merged_origin_lon,
            origin_lat: self.merged_origin_lat,
            x_res: self.merged_x_res,
            y_res: self.merged_y_res,
            values: self.merged_values.clone(),
            start_point: (0, 0),
            metadata: crate::model::Metadata {
                meshcode: format!("merged_{}", self.tiles.len()),
                dem_type: self.tiles.first().unwrap().metadata.dem_type.clone(),
                crs_identifier: self.crs_identifier.clone(),
            },
        }
    }
}
