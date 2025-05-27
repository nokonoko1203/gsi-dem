use anyhow::{Context, Result};
use gdal::raster::Buffer;
use gdal::spatial_ref::SpatialRef;
use gdal::{DriverManager, Metadata};
use std::path::Path;

use crate::model::DemTile;

const NODATA_VALUE: f64 = -9999.0;

#[derive(Default)]
pub struct GeoTiffWriter {}

impl GeoTiffWriter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn write(&self, dem_tile: &DemTile, output_path: &Path) -> Result<()> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DemTile, Metadata};
    use gdal::Dataset;
    use tempfile::TempDir;

    #[test]
    fn test_write_geotiff() {
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
