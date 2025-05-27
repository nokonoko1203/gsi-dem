#[derive(Debug, Clone)]
pub struct DemTile {
    pub rows: usize,
    pub cols: usize,
    pub origin_lon: f64,
    pub origin_lat: f64,
    pub x_res: f64,
    pub y_res: f64,
    pub values: Vec<f32>,
    pub start_point: (usize, usize),
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub meshcode: String,
    pub dem_type: String,
    pub crs_identifier: String,
}

impl DemTile {
    /// タイルの形状を取得
    pub fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// 指定された行列位置の標高値を取得
    pub fn get_value(&self, row: usize, col: usize) -> Option<f32> {
        if row < self.rows && col < self.cols {
            Some(self.values[row * self.cols + col])
        } else {
            None
        }
    }

    /// データが正しいサイズかチェック
    pub fn validate(&self) -> bool {
        self.values.len() == self.rows * self.cols
    }

    /// GeoTransform配列を取得（GDAL形式）
    pub fn geo_transform(&self) -> [f64; 6] {
        // origin_latは既に左上座標として設定されている
        [
            self.origin_lon,
            self.x_res,
            0.0,
            self.origin_lat,
            0.0,
            -self.y_res.abs(), // 負の値（北から南へ）
        ]
    }
    
    /// 座標系識別子からEPSGコードを推定
    pub fn guess_epsg(&self) -> Option<u32> {
        match self.metadata.crs_identifier.as_str() {
            "fguuid:jgd2011.bl" => Some(6668), // JGD2011 / 緯度経度
            "fguuid:jgd2000.bl" => Some(4612), // JGD2000 / 緯度経度
            "fguuid:tokyo.bl" => Some(4301),   // Tokyo / 緯度経度
            _ => None,
        }
    }
}