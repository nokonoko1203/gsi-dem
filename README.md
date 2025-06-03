# japan-dem

基盤地図情報DEM XMLをGeoTIFFに変換するRust製CLIツール

## 概要

本ツールは、国土地理院が提供する基盤地図情報数値標高モデル（DEM）のXMLファイルやZIPアーカイブを、高速並列処理でGeoTIFF形式に変換します。

### 主な機能
- **高速並列処理**: マルチスレッドによる高速変換
- **ZIPファイル対応**: 圧縮されたDEMデータを直接処理
- **タイル結合**: 複数のDEMタイルを1つのGeoTIFFに統合
- **Terrain-RGB出力**: WebGIS用のRGBエンコード標高データ
- **Python バインディング**: QGIS プラグインなどでの利用

### 対応フォーマット
- DEM5A/B/C（5mメッシュ）
- DEM10A/B（10mメッシュ）
- DEM1A（1mメッシュ）
- ZIPアーカイブ（複数XMLファイルを含む）

## インストール

### バイナリのインストール

`${VERSION}` を最新のリリースバージョンに置き換えてください。
最新のバージョンは [GitHub Releases](https://github.com/nokonoko1203/japan-dem/releases) で確認できます。

```bash
$ cargo install japan-dem
or
$ curl --proto '=https' --tlsv1.2 -LsSf https://github.com/nokonoko1203/japan-dem/releases/download/${VERSION}/japan-dem-installer.sh | sh
```

### ソースからのビルド

```bash
# リポジトリのクローン
git clone https://github.com/nokonoko1203/japan-dem.git
cd japan-dem

# リリースビルド
cargo build --release

# バイナリは`target/release/japan-dem`に生成されます
```

## 使用方法

### 基本的な使い方

**単一XMLファイルの変換**
```bash
japan-dem input.xml -o output_dir
```

**ZIPファイルの変換**
```bash
japan-dem input.zip -o output_dir --threads 8 --merge
```

**ディレクトリ一括変換**
```bash
japan-dem input_dir/ -o output_dir --threads 8
```

### コマンドラインオプション

| オプション              | 説明                          | デフォルト |
| ----------------------- | ----------------------------- | ---------- |
| `-o, --output <DIR>`    | 出力ディレクトリ（必須）      | -          |
| `--threads <NUM>`       | 並列処理スレッド数            | CPUコア数  |
| `--merge`               | 複数XMLを1つのGeoTIFFにマージ | false      |
| `--terrain-rgb`         | Terrain-RGB形式で出力         | false      |
| `--min-elevation <NUM>` | 最小標高値（手動設定）        | 自動       |
| `--max-elevation <NUM>` | 最大標高値（手動設定）        | 自動       |

### Terrain-RGB形式での出力

Terrain-RGB形式は、標高値をRGB値にエンコードした8bit形式で、Web地図タイルなどで利用されます。
出力されるGeoTIFFは通常のGeoTIFFと同じ地理参照情報を持ち、GISソフトウェアで利用可能です。

```bash
# Terrain-RGB形式で出力（ファイル名: メッシュコード_terrain_rgb.tif）
japan-dem input.xml -o output_dir --terrain-rgb
```

## Pythonバインディング

QGIS プラグインやPythonスクリプトからの利用が可能です。

### インストール
```bash
# maturin でビルド
maturin develop

# または
pip install japan-dem
```

### 使用例
```python
import japan_dem

# XMLファイルを解析
dem_tile = japan_dem.parse_dem_xml("input.xml")
print(f"Mesh code: {dem_tile.metadata.mesh_code}")
print(f"Size: {dem_tile.rows}x{dem_tile.cols}")

# Terrain-RGB GeoTIFFとして出力
japan_dem.dem_to_terrain_rgb(dem_tile, "output.tif")
```

## ライセンス

MIT License
