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

### ビルド手順

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

## ライセンス

MIT License
