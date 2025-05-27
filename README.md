# gsi-dem

基盤地図情報DEM XMLをGeoTIFFに変換するRust製CLIツール

## 概要

本ツールは、国土地理院が提供する基盤地図情報数値標高モデル（DEM）のXMLファイル（JPGIS-GML 2014／FGD 5.1形式）を、GIS対応ソフトウェアで利用可能なGeoTIFF形式に変換します。

### 対応フォーマット
- DEM5A/B/C（5mメッシュ）
- DEM10A/B（10mメッシュ）
- DEM1A（1mメッシュ）

### 特徴
- ストリーム処理による省メモリ動作
- マルチスレッド対応による高速変換
- GDAL互換のGeoTIFF生成（EPSG:6668 JGD2011）
- NoData値: -9999

## インストール

### 前提条件
- Rust 1.70以上
- GDAL 3.0以上（システムにインストール済みであること）

### crates.io からのインストール（推奨）

```bash
cargo install gsi-dem
```

### ソースからのビルド

```bash
# リポジトリのクローン
git clone https://github.com/yourusername/gsi-dem.git
cd gsi-dem

# リリースビルド
cargo build --release

# バイナリは target/release/gsi-dem に生成されます
```

## 使用方法

### 単一ファイルの変換
```bash
gsi-dem input.xml -o output_dir/
```

### ディレクトリ一括変換
```bash
gsi-dem input_dir/ -o output_dir/ --threads 4
```

### オプション
- `-o, --output <DIR>`: 出力ディレクトリ（必須）
- `-t, --threads <NUM>`: 並列処理スレッド数（デフォルト: CPUコア数）
- `-c, --compress`: Deflate圧縮を有効化
- `--tiling`: タイリングを有効化

## 開発者向け情報

### crates.io への公開

#### 初回設定

```bash
# crates.io アカウントの作成
# https://crates.io でGitHubアカウントでログイン

# APIトークンの取得
# https://crates.io/settings/tokens で新しいトークンを生成

# cargo でログイン
cargo login <your-api-token>
```

#### Cargo.toml の準備

```toml
[package]
name = "gsi-dem"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "基盤地図情報DEM XMLをGeoTIFFに変換するCLIツール"
readme = "README.md"
repository = "https://github.com/yourusername/gsi-dem"
license = "MIT"
keywords = ["gis", "dem", "geotiff", "japan", "gsi"]
categories = ["command-line-utilities", "science::geo"]

# 除外ファイルの設定
exclude = [
    "test_dir/*",
    "test_output/*",
    ".github/*",
    "examples/*",
    "python/*",
]

# バイナリの指定
[[bin]]
name = "gsi-dem"
path = "src/main.rs"
```

#### 公開手順

```bash
# 1. ドライラン（実際には公開されない）
cargo publish --dry-run

# 2. 実際に公開
cargo publish

# 3. バージョンタグを付ける
git tag v0.1.0
git push origin v0.1.0
```

### GitHub Actions での自動リリース

`.github/workflows/release-rust.yml`:

```yaml
name: Release Rust Crate

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    name: Publish to crates.io
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install GDAL
        run: |
          sudo apt-get update
          sudo apt-get install -y gdal-bin libgdal-dev
      
      - name: Publish to crates.io
        run: cargo publish --token ${{ secrets.CARGO_REGISTRY_TOKEN }}
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

### バージョンアップ時の注意

```bash
# バージョンを更新
cargo set-version 0.2.0

# 変更をコミット
git add Cargo.toml Cargo.lock
git commit -m "Bump version to 0.2.0"

# タグを付けて公開
git tag v0.2.0
git push origin main v0.2.0
cargo publish
```

## ライセンス

MIT License
