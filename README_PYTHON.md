# GSI DEM Python バインディング

PyO3 で構築された国土地理院 DEM XML パーサーの Python バインディングです。

## インストール

### ソースからのインストール

```bash
# uv のインストール（まだの場合）
curl -LsSf https://astral.sh/uv/install.sh | sh

# maturin のインストールと Python モジュールのビルド
uv pip install maturin
uv run maturin develop --release --features python
```

### 開発環境

```bash
# uv で仮想環境を作成
uv venv

# maturin のインストール
uv pip install maturin

# 開発モードでビルド
uv run maturin develop --features python
```

## 使い方

### 基本的な例

```python
import gsi_dem

# DEM XML ファイルをパース
dem_tile = gsi_dem.parse_dem_xml('path/to/dem.xml')

# プロパティにアクセス
print(f"形状: {dem_tile.shape}")  # (行数, 列数)
print(f"原点: ({dem_tile.origin_lon}, {dem_tile.origin_lat})")
print(f"解像度: {dem_tile.x_res} x {dem_tile.y_res}")
print(f"メッシュコード: {dem_tile.metadata.mesh_code}")
print(f"座標系: {dem_tile.metadata.crs_identifier}")

# 標高値にアクセス
print(f"値の数: {len(dem_tile.values)}")
print(f"最初の値: {dem_tile.values[0]}")
```

### NumPy との連携

```python
import numpy as np
import gsi_dem

dem_tile = gsi_dem.parse_dem_xml('path/to/dem.xml')

# NumPy 配列に変換
# 注意: 部分データの場合は start_point を考慮する必要があります
data = np.full((dem_tile.rows, dem_tile.cols), -9999.0, dtype=np.float32)
start_x, start_y = dem_tile.start_point

# 実際の値で配列を埋める
idx = 0
for row in range(start_y, dem_tile.rows):
    for col in range(start_x if row == start_y else 0, dem_tile.cols):
        if idx < len(dem_tile.values):
            data[row, col] = dem_tile.values[idx]
            idx += 1
```

### QGIS プラグインでの統合

QGIS プラグインでバインディングを使用する完全な例は `examples/qgis_plugin_example.py` を参照してください。

## API リファレンス

### 関数

#### `parse_dem_xml(path: str) -> DemTile`
国土地理院 DEM XML ファイルをパースして DemTile オブジェクトを返します。

- **パラメータ:**
  - `path`: XML ファイルへのパス
- **戻り値:** DemTile オブジェクト
- **例外:** パースに失敗した場合は IOError

### クラス

#### `DemTile`
パースされた DEM データを表します。

**属性:**
- `rows: int` - 行数
- `cols: int` - 列数
- `origin_lon: float` - 原点の経度（左下隅）
- `origin_lat: float` - 原点の緯度（左下隅）
- `x_res: float` - X 方向の解像度（度）
- `y_res: float` - Y 方向の解像度（度）
- `values: List[float]` - 標高値のリスト
- `start_point: Tuple[int, int]` - 部分データの開始点 (x, y)
- `metadata: Metadata` - 関連するメタデータ

**プロパティ:**
- `shape: Tuple[int, int]` - (行数, 列数) を返します

#### `Metadata`
メタデータ情報を含みます。

**属性:**
- `mesh_code: str` - メッシュコード
- `dem_type: str` - DEM タイプ（例: "5A", "10B"）
- `crs_identifier: str` - 座標参照系識別子

## テスト

```bash
# Python テストを実行
uv run pytest python/tests/

# または直接実行
uv run python python/tests/test_parser.py
```

## 配布用のビルド

### ローカルビルド

```bash
# 現在のプラットフォーム用のwheel をビルド
uv run maturin build --release --features python

# ユニバーサル wheel をビルド（可能な場合）
uv run maturin build --release --features python --universal2

# wheel は target/wheels/ に出力されます
```

### PyPI への公開

#### 初回設定

```bash
# PyPI アカウントの作成
# https://pypi.org/account/register/ でアカウントを作成

# API トークンの取得
# https://pypi.org/manage/account/token/ でトークンを生成

# maturin での認証設定（以下のいずれか）
# 方法1: 環境変数を使用
export MATURIN_PYPI_TOKEN="pypi-AgEIcH..."

# 方法2: パスワードを使用（非推奨）
export MATURIN_USERNAME="__token__"
export MATURIN_PASSWORD="pypi-AgEIcH..."
```

#### ビルドと公開

```bash
# 1. すべてのプラットフォーム用のwheel をビルド
# Linux (manylinux)
uv run maturin build --release --features python --compatibility manylinux2014

# macOS (Intel + Apple Silicon)
uv run maturin build --release --features python --universal2

# Windows
uv run maturin build --release --features python

# 2. TestPyPI でテスト（推奨）
uv run maturin publish --features python --repository testpypi

# テストインストール
uv pip install --index-url https://test.pypi.org/simple/ gsi-dem

# 3. 本番 PyPI に公開
uv run maturin publish --features python
```

#### GitHub Actions での自動リリース

`.github/workflows/release.yml` を作成して自動化できます：

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    name: Release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64
          - os: windows-latest
            target: x86_64
          - os: macos-latest
            target: x86_64
          - os: macos-latest
            target: aarch64
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v4
        with:
          python-version: '3.10'
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      
      - name: Install maturin
        run: pip install maturin
      
      - name: Build wheels
        run: maturin build --release --features python --out dist
      
      - name: Upload wheels
        uses: actions/upload-artifact@v3
        with:
          name: wheels
          path: dist

  publish:
    name: Publish
    runs-on: ubuntu-latest
    needs: [release]
    steps:
      - uses: actions/download-artifact@v3
        with:
          name: wheels
      
      - name: Publish to PyPI
        uses: pypa/gh-action-pypi-publish@release/v1
        with:
          password: ${{ secrets.PYPI_API_TOKEN }}
```

### インストール確認

PyPI に公開後は、以下でインストールできます：

```bash
# pip でのインストール
pip install gsi-dem

# uv でのインストール
uv pip install gsi-dem
```
