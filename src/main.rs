use anyhow::Result;
use clap::Parser;
use rayon::ThreadPoolBuilder;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, info};
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 入力XMLファイル、ZIPファイル、またはディレクトリ
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// 出力ディレクトリ
    #[arg(short, long, value_name = "DIR")]
    output: PathBuf,

    /// 並列処理スレッド数（デフォルト: CPUコア数）
    #[arg(short, long)]
    threads: Option<usize>,

    /// 複数のDEMタイルを結合して出力
    #[arg(long)]
    merge: bool,
}

fn main() -> Result<()> {
    // ログの初期化
    tracing_subscriber::fmt::init();

    // CLI引数の解析
    let args = Args::parse();

    // スレッドプールの設定
    if let Some(threads) = args.threads {
        ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build thread pool");
    }

    // 出力ディレクトリの作成
    fs::create_dir_all(&args.output)?;

    // 入力パスの処理
    if args.input.is_file() {
        let ext = args
            .input
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        match ext {
            "zip" => {
                // ZIPファイルの処理
                info!("Processing ZIP file: {:?}", args.input);
                process_zip_file(&args.input, &args)?;
            }
            "xml" => {
                // XMLファイルの処理
                info!("Processing XML file: {:?}", args.input);
                process_file(&args.input, &args)?;
            }
            _ => {
                error!("Unsupported file type: {:?}", args.input);
                anyhow::bail!("Input file must be .xml or .zip");
            }
        }
    } else if args.input.is_dir() {
        // ディレクトリの処理
        info!("Processing directory: {:?}", args.input);
        process_directory(&args.input, &args)?;
    } else {
        error!("Invalid input path: {:?}", args.input);
        anyhow::bail!("Input path must be a file or directory");
    }

    Ok(())
}

fn process_file(path: &Path, args: &Args) -> Result<()> {
    info!("Processing file: {:?}", path);

    use japan_dem::parser::parse_dem_xml;
    use japan_dem::writer::GeoTiffWriter;
    use std::fs::File;
    use std::io::BufReader;

    // XMLファイルを解析
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let dem_tile = parse_dem_xml(reader)?;

    info!(
        "Parsed successfully: {} ({}x{})",
        dem_tile.metadata.meshcode, dem_tile.rows, dem_tile.cols
    );

    // 出力ファイル名を生成（メッシュコード.tif）
    let output_filename = format!("{}.tif", dem_tile.metadata.meshcode);
    let output_path = args.output.join(&output_filename);

    // GeoTIFFに変換
    let writer = GeoTiffWriter::new();
    writer.write(&dem_tile, &output_path)?;

    info!("Written GeoTIFF: {:?}", output_path);

    Ok(())
}

fn process_directory(dir: &Path, args: &Args) -> Result<()> {
    use rayon::prelude::*;

    // XML/ZIPファイルを再帰的に収集
    let input_files = collect_input_files(dir)?;
    info!("Found {} input files (XML/ZIP)", input_files.len());

    // 並列処理でファイルを変換
    let results: Vec<Result<()>> = input_files
        .par_iter()
        .map(|(path, file_type)| match file_type {
            FileType::Xml => process_file(path, args),
            FileType::Zip => process_zip_file(path, args),
        })
        .collect();

    // エラーをチェック
    let mut errors = Vec::new();
    for (i, result) in results.into_iter().enumerate() {
        if let Err(e) = result {
            errors.push(format!("{}: {}", input_files[i].0.display(), e));
        }
    }

    if !errors.is_empty() {
        error!("Failed to process {} files:", errors.len());
        for err in &errors {
            error!("  {}", err);
        }
        anyhow::bail!("{} files failed to process", errors.len());
    }

    Ok(())
}

fn collect_input_files(dir: &Path) -> Result<Vec<(std::path::PathBuf, FileType)>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // 再帰的に探索
            let mut sub_files = collect_input_files(&path)?;
            files.append(&mut sub_files);
        } else {
            match path.extension().and_then(|s| s.to_str()) {
                Some("xml") => files.push((path, FileType::Xml)),
                Some("zip") => files.push((path, FileType::Zip)),
                _ => {}
            }
        }
    }

    Ok(files)
}

#[derive(Debug, Clone, Copy)]
enum FileType {
    Xml,
    Zip,
}

fn process_zip_file(path: &Path, args: &Args) -> Result<()> {
    use japan_dem::writer::GeoTiffWriter;
    use japan_dem::{MergedDemTile, ZipHandler};

    let handler = ZipHandler::new(path);
    let tiles = handler.process_all_tiles()?;

    if args.merge && tiles.len() > 1 {
        // タイルを結合して出力
        info!("Merging {} tiles", tiles.len());
        let merged = MergedDemTile::from_tiles(tiles)?;
        let dem_tile = merged.to_dem_tile();

        // 出力ファイル名を生成（ZIPファイル名から.zipを除いたもの）
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("merged");
        let output_filename = format!("{}.tif", stem);
        let output_path = args.output.join(&output_filename);

        // GeoTIFFに変換
        let writer = GeoTiffWriter::new();
        writer.write(&dem_tile, &output_path)?;

        info!("Written merged GeoTIFF: {:?}", output_path);
    } else {
        // 各タイルを個別に出力
        info!("Writing {} tiles individually", tiles.len());
        let writer = GeoTiffWriter::new();

        for tile in tiles {
            let output_filename = format!("{}.tif", tile.metadata.meshcode);
            let output_path = args.output.join(&output_filename);

            writer.write(&tile, &output_path)?;
            info!("Written GeoTIFF: {:?}", output_path);
        }
    }

    Ok(())
}
