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
    /// 入力XMLファイルまたはディレクトリ
    #[arg(value_name = "INPUT")]
    input: PathBuf,

    /// 出力ディレクトリ
    #[arg(short, long, value_name = "DIR")]
    output: PathBuf,

    /// 並列処理スレッド数（デフォルト: CPUコア数）
    #[arg(short, long)]
    threads: Option<usize>,
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
        // 単一ファイルの処理
        info!("Processing single file: {:?}", args.input);
        process_file(&args.input, &args)?;
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

    use gsi_dem::parser::parse_dem_xml;
    use gsi_dem::writer::GeoTiffWriter;
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

    // XMLファイルを再帰的に収集
    let xml_files = collect_xml_files(dir)?;
    info!("Found {} XML files", xml_files.len());

    // 並列処理でファイルを変換
    let results: Vec<Result<()>> = xml_files
        .par_iter()
        .map(|file| process_file(file, args))
        .collect();

    // エラーをチェック
    let mut errors = Vec::new();
    for (i, result) in results.into_iter().enumerate() {
        if let Err(e) = result {
            errors.push(format!("{}: {}", xml_files[i].display(), e));
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

fn collect_xml_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut xml_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // 再帰的に探索
            let mut sub_files = collect_xml_files(&path)?;
            xml_files.append(&mut sub_files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("xml") {
            xml_files.push(path);
        }
    }

    Ok(xml_files)
}
