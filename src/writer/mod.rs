use anyhow::{Context, Result};
use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::{DriverManager, Metadata};
use std::path::Path;

use crate::model::DemTile;
use crate::terrain_rgb::{elevation_to_rgb, TerrainRgbConfig};

const NODATA_VALUE: f64 = -9999.0;

#[derive(Default)]
pub struct GeoTiffWriter {}

impl GeoTiffWriter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn write(&self, dem_tile: &DemTile, output_path: &Path) -> Result<()> {
        self.write_standard(dem_tile, output_path)
    }

    pub fn write_terrain_rgb(
        &self,
        dem_tile: &DemTile,
        output_path: &Path,
        _config: &TerrainRgbConfig,
    ) -> Result<()> {
        let (rows, cols) = dem_tile.shape();

        tracing::info!(
            "Converting DEM to Terrain-RGB GeoTIFF: {} x {} pixels",
            cols,
            rows
        );

        // GTiffドライバーを取得
        let driver =
            DriverManager::get_driver_by_name("GTiff").context("Failed to get GTiff driver")?;

        // 8-bit RGB GeoTIFFを作成
        let mut dataset = driver
            .create_with_band_type::<u8, _>(
                output_path,
                cols,
                rows,
                3, // RGB 3バンド
            )
            .context("Failed to create dataset")?;

        self.set_geo_metadata(&mut dataset, dem_tile)?;

        // RGBデータを準備
        let mut r_band = vec![0u8; cols * rows];
        let mut g_band = vec![0u8; cols * rows];
        let mut b_band = vec![0u8; cols * rows];

        for (i, &elevation) in dem_tile.values.iter().enumerate() {
            if elevation == -9999.0 {
                r_band[i] = 0;
                g_band[i] = 0;
                b_band[i] = 0;
            } else {
                let (r, g, b) = elevation_to_rgb(elevation);
                r_band[i] = r;
                g_band[i] = g;
                b_band[i] = b;
            }
        }

        // バンドにデータを書き込み
        self.write_rgb_bands(&mut dataset, cols, rows, r_band, g_band, b_band)?;

        Ok(())
    }

    fn write_standard(&self, dem_tile: &DemTile, output_path: &Path) -> Result<()> {
        // GTiffドライバーを取得
        let driver =
            DriverManager::get_driver_by_name("GTiff").context("Failed to get GTiff driver")?;

        // データセットを作成
        let (rows, cols) = dem_tile.shape();
        let mut dataset = driver
            .create_with_band_type::<f32, _>(
                output_path,
                cols,
                rows,
                1, // バンド数
            )
            .context("Failed to create dataset")?;

        // ジオトランスフォームを設定
        dataset
            .set_geo_transform(&dem_tile.geo_transform())
            .context("Failed to set geo transform")?;

        // 座標系を設定
        if let Some(epsg) = dem_tile.guess_epsg() {
            let srs = SpatialRef::from_epsg(epsg)
                .context(format!("Failed to create SpatialRef from EPSG:{}", epsg))?;
            let wkt = srs
                .to_wkt()
                .context("Failed to convert SpatialRef to WKT")?;
            dataset
                .set_projection(&wkt)
                .context("Failed to set projection")?;
        } else {
            // EPSGコードが推定できない場合は警告
            eprintln!(
                "Warning: Unknown CRS identifier: {}",
                dem_tile.metadata.crs_identifier
            );
        }

        // バンドにデータを書き込み
        let mut band = dataset.rasterband(1).context("Failed to get raster band")?;

        // NoData値を設定
        band.set_no_data_value(Some(NODATA_VALUE))
            .context("Failed to set no data value")?;

        // データを書き込み（GDALは行優先順を期待）
        let mut buffer = Buffer::new((cols, rows), dem_tile.values.clone());
        band.write((0, 0), (cols, rows), &mut buffer)
            .context("Failed to write raster data")?;

        // メタデータを設定（オプション）
        dataset
            .set_metadata_item("MESHCODE", &dem_tile.metadata.meshcode, "")
            .context("Failed to set meshcode metadata")?;
        dataset
            .set_metadata_item("DEM_TYPE", &dem_tile.metadata.dem_type, "")
            .context("Failed to set dem_type metadata")?;

        Ok(())
    }

    fn set_geo_metadata(&self, dataset: &mut gdal::Dataset, dem_tile: &DemTile) -> Result<()> {
        // ジオトランスフォームを設定
        dataset
            .set_geo_transform(&dem_tile.geo_transform())
            .context("Failed to set geo transform")?;

        // 座標系を設定
        if let Some(epsg) = dem_tile.guess_epsg() {
            let srs = SpatialRef::from_epsg(epsg)
                .context(format!("Failed to create SpatialRef from EPSG:{}", epsg))?;
            let wkt = srs
                .to_wkt()
                .context("Failed to convert SpatialRef to WKT")?;
            dataset
                .set_projection(&wkt)
                .context("Failed to set projection")?;
        } else {
            // EPSGコードが推定できない場合は警告
            eprintln!(
                "Warning: Unknown CRS identifier: {}",
                dem_tile.metadata.crs_identifier
            );
        }

        // メタデータを設定
        dataset
            .set_metadata_item("MESHCODE", &dem_tile.metadata.meshcode, "")
            .context("Failed to set meshcode metadata")?;
        dataset
            .set_metadata_item("DEM_TYPE", &dem_tile.metadata.dem_type, "")
            .context("Failed to set dem_type metadata")?;

        Ok(())
    }

    fn write_rgb_bands(
        &self,
        dataset: &mut gdal::Dataset,
        cols: usize,
        rows: usize,
        r_band: Vec<u8>,
        g_band: Vec<u8>,
        b_band: Vec<u8>,
    ) -> Result<()> {
        // バンド1 (R)
        let mut band = dataset
            .rasterband(1)
            .context("Failed to get raster band 1")?;
        let mut buffer = Buffer::new((cols, rows), r_band);
        band.write((0, 0), (cols, rows), &mut buffer)
            .context("Failed to write R band")?;

        // バンド2 (G)
        let mut band = dataset
            .rasterband(2)
            .context("Failed to get raster band 2")?;
        let mut buffer = Buffer::new((cols, rows), g_band);
        band.write((0, 0), (cols, rows), &mut buffer)
            .context("Failed to write G band")?;

        // バンド3 (B)
        let mut band = dataset
            .rasterband(3)
            .context("Failed to get raster band 3")?;
        let mut buffer = Buffer::new((cols, rows), b_band);
        band.write((0, 0), (cols, rows), &mut buffer)
            .context("Failed to write B band")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DemTile, Metadata};
    use gdal::Dataset;
    use std::sync::Once;
    use tempfile::TempDir;

    static INIT: Once = Once::new();

    fn init_gdal() -> bool {
        INIT.call_once(|| {
            // GDALの初期化を試みる
            // bundled版では自動的に初期化されるはず
        });

        // GTiffドライバーが利用可能かチェック
        DriverManager::get_driver_by_name("GTiff").is_ok()
    }

    #[test]
    fn test_write_geotiff() {
        if !init_gdal() {
            eprintln!("Skipping test: GTiff driver not available in bundled GDAL");
            return;
        }
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test.tif");

        let dem_tile = create_test_tile();
        let writer = GeoTiffWriter::new();

        writer.write(&dem_tile, &output_path).unwrap();

        // ファイルが作成されたことを確認
        assert!(output_path.exists());

        // GDALで読み返してテスト
        let dataset = Dataset::open(&output_path).unwrap();
        assert_eq!(dataset.raster_size(), (3, 2));

        let transform = dataset.geo_transform().unwrap();
        assert_eq!(transform[0], 135.0); // origin_lon
        assert_eq!(transform[1], 0.001); // x_res

        let band = dataset.rasterband(1).unwrap();
        let nodata = band.no_data_value().unwrap();
        assert_eq!(nodata, NODATA_VALUE);
    }

    #[test]
    fn test_consistent_output_shapes() {
        if !init_gdal() {
            eprintln!("Skipping test: GTiff driver not available in bundled GDAL");
            return;
        }
        use crate::terrain_rgb::TerrainRgbConfig;

        let temp_dir = TempDir::new().unwrap();
        let standard_path = temp_dir.path().join("standard.tif");
        let terrain_rgb_path = temp_dir.path().join("terrain_rgb.tif");

        let dem_tile = create_test_tile();
        let writer = GeoTiffWriter::new();

        // 通常のGeoTIFF出力
        writer.write(&dem_tile, &standard_path).unwrap();

        // Terrain-RGB出力
        let config = TerrainRgbConfig {
            min_elevation: None,
            max_elevation: None,
        };
        writer
            .write_terrain_rgb(&dem_tile, &terrain_rgb_path, &config)
            .unwrap();

        // 両方のファイルをGDALで読み返す
        let standard_dataset = Dataset::open(&standard_path).unwrap();
        let terrain_rgb_dataset = Dataset::open(&terrain_rgb_path).unwrap();

        // 形状が一致することを確認
        assert_eq!(
            standard_dataset.raster_size(),
            terrain_rgb_dataset.raster_size()
        );
        assert_eq!(standard_dataset.raster_size(), (3, 2)); // (cols, rows)

        // ジオトランスフォームが一致することを確認
        let standard_transform = standard_dataset.geo_transform().unwrap();
        let terrain_rgb_transform = terrain_rgb_dataset.geo_transform().unwrap();

        for i in 0..6 {
            assert!(
                (standard_transform[i] - terrain_rgb_transform[i]).abs() < 1e-10,
                "Geo transforms differ at index {}: {} vs {}",
                i,
                standard_transform[i],
                terrain_rgb_transform[i]
            );
        }

        // 座標系が一致することを確認
        let standard_proj = standard_dataset.projection();
        let terrain_rgb_proj = terrain_rgb_dataset.projection();
        assert_eq!(standard_proj, terrain_rgb_proj);

        // バンド数が異なることを確認（標準=1, Terrain-RGB=3）
        assert_eq!(standard_dataset.raster_count(), 1);
        assert_eq!(terrain_rgb_dataset.raster_count(), 3);
    }

    fn create_test_tile() -> DemTile {
        DemTile {
            rows: 2,
            cols: 3,
            origin_lon: 135.0,
            origin_lat: 35.0,
            x_res: 0.001,
            y_res: 0.001,
            values: vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0],
            start_point: (0, 0),
            metadata: Metadata {
                meshcode: "12345678".to_string(),
                dem_type: "1mメッシュ（標高）".to_string(),
                crs_identifier: "fguuid:jgd2011.bl".to_string(),
            },
        }
    }
}
