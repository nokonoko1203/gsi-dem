# japan DEM Pythonバインディング

PyO3で構築された国土地理院のDEM（XML）パーサーのPythonバインディングです

## インストール

### ソースからのインストール

```bash
# uvのインストール（まだの場合）
curl -LsSf https://astral.sh/uv/install.sh | sh

# maturinのインストールとPythonモジュールのビルド
uv pip install maturin
uv run maturin develop --release --features python
```

### 開発環境

```bash
# uvで仮想環境を作成
uv venv

# maturinのインストール
uv pip install maturin

# 開発モードでビルド
uv run maturin develop --features python
```

## 使い方

### 基本的な例

```python
import japan_dem

# DEM XMLファイルをパース
dem_tile = japan_dem.parse_dem_xml('path/to/dem.xml')

# プロパティにアクセス
print(f"形状: {dem_tile.shape}")
print(f"原点: ({dem_tile.origin_lon}, {dem_tile.origin_lat})")
print(f"解像度: {dem_tile.x_res} x {dem_tile.y_res}")
print(f"メッシュコード: {dem_tile.metadata.mesh_code}")
print(f"座標系: {dem_tile.metadata.crs_identifier}")

# 標高値にアクセス
print(f"値の数: {len(dem_tile.values)}")
print(f"最初の値: {dem_tile.values[0]}")
```

### numpyとの連携

```python
import numpy as np
import japan_dem

dem_tile = japan_dem.parse_dem_xml('path/to/dem.xml')

# numpy配列に変換
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

## API リファレンス

### 関数

#### `parse_dem_xml(path: str) -> DemTile`
国土地理院のDEMをパースして DemTile オブジェクトを返します。

- **パラメータ:**
  - `path`: XMLファイルへのパス
- **戻り値:** DemTileオブジェクト
- **例外:** パースに失敗した場合はIOError

### クラス

#### `DemTile`
パースされたDEMデータを表します。

**属性:**
- `rows: int` - 行数
- `cols: int` - 列数
- `origin_lon: float` - 原点の経度（左下隅）
- `origin_lat: float` - 原点の緯度（左下隅）
- `x_res: float` - X方向の解像度（度）
- `y_res: float` - Y方向の解像度（度）
- `values: List[float]` - 標高値のリスト
- `start_point: Tuple[int, int]` - 部分データの開始点 (x, y)
- `metadata: Metadata` - 関連するメタデータ

**プロパティ:**
- `shape: Tuple[int, int]` - (行数, 列数) を返します

#### `Metadata`
メタデータ情報を含みます。

**属性:**
- `mesh_code: str` - メッシュコード
- `dem_type: str` - DEMタイプ（例: "5A", "10B"）
- `crs_identifier: str` - 座標参照系識別子

## テスト

```bash
# Pythonテストを実行
uv run pytest python/tests/

# または直接実行
uv run python python/tests/test_parser.py
```

## 配布用のビルド

### ローカルビルド

```bash
# 現在のプラットフォーム用のwheelをビルド
uv run maturin build --release --features python

# wheelは`target/wheels/`に出力されます
```

