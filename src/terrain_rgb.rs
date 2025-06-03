use crate::model::DemTile;

#[derive(Debug, Clone, Copy)]
pub struct RgbDepth;

#[derive(Debug, Clone, Default)]
pub struct TerrainRgbConfig {
    pub min_elevation: Option<f32>,
    pub max_elevation: Option<f32>,
}

pub fn elevation_to_rgb(elevation: f32) -> (u8, u8, u8) {
    let base_elevation = -10000.0;
    let interval = 0.1;

    let encoded = ((elevation - base_elevation) / interval).round() as i32;

    let r = ((encoded >> 16) & 0xFF) as u8;
    let g = ((encoded >> 8) & 0xFF) as u8;
    let b = (encoded & 0xFF) as u8;

    (r, g, b)
}

pub fn rgb_to_elevation(r: u8, g: u8, b: u8) -> f32 {
    let base_elevation = -10000.0;
    let interval = 0.1;

    let encoded = ((r as i32) << 16) | ((g as i32) << 8) | (b as i32);

    base_elevation + (encoded as f32) * interval
}

pub fn find_elevation_range(tiles: &[DemTile]) -> (f32, f32) {
    let mut min_elevation = f32::INFINITY;
    let mut max_elevation = f32::NEG_INFINITY;

    for tile in tiles {
        for &value in &tile.values {
            if value != -9999.0 {
                min_elevation = min_elevation.min(value);
                max_elevation = max_elevation.max(value);
            }
        }
    }

    (min_elevation, max_elevation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_rgb_conversion() {
        let test_elevations = vec![0.0, 100.0, 1000.0, -100.0, 8848.0];

        for elevation in test_elevations {
            let (r, g, b) = elevation_to_rgb(elevation);
            let decoded = rgb_to_elevation(r, g, b);

            assert!(
                (elevation - decoded).abs() < 0.1,
                "Elevation {} decoded to {} (diff: {})",
                elevation,
                decoded,
                (elevation - decoded).abs()
            );
        }
    }

    #[test]
    fn test_negative_elevation_handling() {
        let negative_elevations = vec![-1000.0, -500.0, -10.0];

        for elevation in negative_elevations {
            let (r, g, b) = elevation_to_rgb(elevation);
            let decoded = rgb_to_elevation(r, g, b);

            assert!(
                (elevation - decoded).abs() < 0.1,
                "Negative elevation {} failed: decoded to {}",
                elevation,
                decoded
            );
        }
    }
}
