# japan-dem

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
- GDAL互換のGeoTIFF生成
- NoData値: -9999

## インストール

### crates.io からのインストール（推奨）

```bash
cargo install japan-dem
```

### ソースからのビルド

```bash
# リポジトリのクローン
git clone https://github.com/nokonoko1203/japan-dem.git
cd japan-dem

# リリースビルド
cargo build --release

# バイナリは target/release/japan-dem に生成されます
```

## 使用方法

### 単一(zip/xml)ファイルの変換
```bash
japan-dem input.zip -o test_output --threads 4 --merge
```

### ディレクトリ一括変換
```bash
japan-dem input_dir/ -o output_dir --threads 4 --merge
```

### オプション
- `-o, --output <DIR>`: 出力ディレクトリ（必須）
- `--threads <NUM>`: 並列処理スレッド数（デフォルト: CPUコア数）
- `--merge`: 複数XMLを1つのGeoTIFFにマージ
- `--terrain-rgb`: Terrain-RGB形式のGeoTIFFで出力（標高値をRGB値にエンコード）
- `--rgb-depth <DEPTH>`: Terrain-RGBの出力形式（8または16、デフォルト: 8）

### Terrain-RGB形式での出力

Terrain-RGB形式は、標高値をRGB値にエンコードした形式で、Web地図タイルなどで利用されます。
出力されるGeoTIFFは通常のGeoTIFFと同じ地理参照情報を持ち、GISソフトウェアで利用可能です。

```bash
# 8bit Terrain-RGB形式で出力（ファイル名: メッシュコード_terrain_rgb.tif）
japan-dem input.xml -o output_dir --terrain-rgb

# 16bit高精度Terrain-RGB形式で出力
japan-dem input.zip -o output_dir --terrain-rgb --rgb-depth 16
```
