use std::path::Path;
use std::time::Instant;
use anyhow::Result;
use japan_dem::ZipHandler;

fn main() -> Result<()> {
    // 環境変数でテスト用ZIPファイルのパスを指定
    let test_zip = std::env::var("TEST_ZIP_PATH")
        .unwrap_or_else(|_| "test_dir/FG-GML-644130-DEM1A-20250227.zip".to_string());
    
    let zip_path = Path::new(&test_zip);
    
    if !zip_path.exists() {
        eprintln!("Test ZIP file not found: {}", test_zip);
        eprintln!("Set TEST_ZIP_PATH environment variable to specify test file");
        return Ok(());
    }
    
    println!("Benchmarking ZIP processing: {}", test_zip);
    
    // 1回目の計測（キャッシュなし）
    let start = Instant::now();
    let handler = ZipHandler::new(zip_path);
    let tiles1 = handler.process_all_tiles()?;
    let duration1 = start.elapsed();
    
    println!("First run: {:?} ({} tiles)", duration1, tiles1.len());
    
    // 2回目の計測（キャッシュあり）
    let start = Instant::now();
    let handler = ZipHandler::new(zip_path);
    let tiles2 = handler.process_all_tiles()?;
    let duration2 = start.elapsed();
    
    println!("Second run: {:?} ({} tiles)", duration2, tiles2.len());
    
    // 3回目の計測
    let start = Instant::now();
    let handler = ZipHandler::new(zip_path);
    let tiles3 = handler.process_all_tiles()?;
    let duration3 = start.elapsed();
    
    println!("Third run: {:?} ({} tiles)", duration3, tiles3.len());
    
    // 平均時間を計算
    let avg_duration = (duration1 + duration2 + duration3) / 3;
    println!("Average processing time: {:?}", avg_duration);
    
    if let Some(first_tile) = tiles1.first() {
        println!("Sample tile info:");
        println!("  Mesh code: {}", first_tile.metadata.meshcode);
        println!("  Size: {}x{}", first_tile.rows, first_tile.cols);
        println!("  Values count: {}", first_tile.values.len());
        println!("  Start point: {:?}", first_tile.start_point);
    }
    
    Ok(())
}