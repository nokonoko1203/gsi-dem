# QGIS プラグインでの japan-dem 統合ガイド

QGISプラグインから `japan-dem` Python バインディングを使用する方法について説明します。

## 配布方法の選択

### 方法1: ビルド済みwheelファイルをプラグインに同梱（推奨）

最も確実な方法は、各プラットフォーム用のwheelファイルをビルド済みの状態でプラグインに含めることです。

#### ディレクトリ構造

```
your_qgis_plugin/
├── __init__.py
├── metadata.txt
├── your_plugin.py
├── libs/                          # ライブラリ格納用ディレクトリ
│   ├── japan_dem-0.1.0-cp39-cp39-win_amd64.whl
│   ├── japan_dem-0.1.0-cp39-cp39-macosx_10_12_x86_64.whl
│   ├── japan_dem-0.1.0-cp39-cp39-macosx_11_0_arm64.whl
│   └── japan_dem-0.1.0-cp39-cp39-linux_x86_64.whl
└── utils/
    └── installer.py              # インストールヘルパー
```

#### インストールヘルパー (utils/installer.py)

```python
import os
import sys
import platform
import subprocess
from pathlib import Path

def get_wheel_filename():
    """プラットフォームに応じた wheel ファイル名を返す"""
    system = platform.system().lower()
    machine = platform.machine().lower()

    if system == 'windows':
        return 'japan_dem-0.1.0-cp39-cp39-win_amd64.whl'
    elif system == 'darwin':  # macOS
        if machine == 'arm64':
            return 'japan_dem-0.1.0-cp39-cp39-macosx_11_0_arm64.whl'
        else:
            return 'japan_dem-0.1.0-cp39-cp39-macosx_10_12_x86_64.whl'
    elif system == 'linux':
        return 'japan_dem-0.1.0-cp39-cp39-linux_x86_64.whl'
    else:
        raise RuntimeError(f"Unsupported platform: {system}")

def install_japan_dem():
    """japan-dem をローカルにインストール"""
    try:
        import japan_dem
        return True  # すでにインストール済み
    except ImportError:
        pass

    # プラグインディレクトリを取得
    plugin_dir = Path(__file__).parent.parent
    libs_dir = plugin_dir / 'libs'

    # 適切な wheel ファイルを選択
    wheel_file = libs_dir / get_wheel_filename()

    if not wheel_file.exists():
        raise FileNotFoundError(f"Wheel file not found: {wheel_file}")

    # pip でインストール
    subprocess.check_call([
        sys.executable, '-m', 'pip', 'install',
        '--user', '--force-reinstall', str(wheel_file)
    ])

    return True
```

#### プラグインの初期化時に自動インストール

```python
# __init__.py
def classFactory(iface):
    from .your_plugin import YourPlugin

    # japan-dem のインストールを試みる
    try:
        from .utils.installer import install_japan_dem
        install_japan_dem()
    except Exception as e:
        from qgis.core import QgsMessageLog, Qgis
        QgsMessageLog.logMessage(
            f"Failed to install japan-dem: {str(e)}",
            'YourPlugin',
            Qgis.Warning
        )

    return YourPlugin(iface)
```

### 方法2: ソースコードをプラグインに含める

Rust/Python混合プロジェクトを直接含めるのは複雑なので、ビルド済みの Python 拡張モジュール（.so/.pyd/.dylib）を含める方法です。

#### ディレクトリ構造

```
your_qgis_plugin/
├── __init__.py
├── metadata.txt
├── your_plugin.py
└── japan_dem/                      # ビルド済みモジュール
    ├── __init__.py
    ├── japan_dem.cpython-39-darwin.so     # macOS用
    ├── japan_dem.cp39-win_amd64.pyd       # Windows用
    └── japan_dem.cpython-39-x86_64-linux-gnu.so  # Linux用
```

#### プラットフォーム別の動的ロード

```python
# japan_dem/__init__.py
import platform
import sys
from pathlib import Path

# プラットフォームに応じた拡張モジュールをロード
system = platform.system()
module_dir = Path(__file__).parent

if system == 'Windows':
    module_path = module_dir / 'japan_dem.cp39-win_amd64.pyd'
elif system == 'Darwin':
    module_path = module_dir / 'japan_dem.cpython-39-darwin.so'
else:  # Linux
    module_path = module_dir / 'japan_dem.cpython-39-x86_64-linux-gnu.so'

# モジュールをインポート
import importlib.util
spec = importlib.util.spec_from_file_location("japan_dem", module_path)
japan_dem = importlib.util.module_from_spec(spec)
sys.modules["japan_dem"] = japan_dem
spec.loader.exec_module(japan_dem)

# 公開 API
from japan_dem import parse_dem_xml, DemTile, Metadata
__all__ = ['parse_dem_xml', 'DemTile', 'Metadata']
```

## ビルド手順

### 各プラットフォーム用のビルド

```bash
# 1. 開発環境の準備
uv venv
uv pip install maturin

# 2. Windows 用ビルド (Windows 上で実行)
uv run maturin build --release --features python --out dist

# 3. macOS 用ビルド (macOS 上で実行)
# Intel Mac
uv run maturin build --release --features python --target x86_64-apple-darwin --out dist
# Apple Silicon
uv run maturin build --release --features python --target aarch64-apple-darwin --out dist

# 4. Linux 用ビルド (Linux 上で実行)
uv run maturin build --release --features python --manylinux 2014 --out dist
```

### GitHub Actions での自動ビルド

`.github/workflows/build-wheels.yml`:

```yaml
name: Build Wheels for QGIS

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        python-version: ['3.9']  # QGIS のPythonバージョンに合わせる

    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: ${{ matrix.python-version }}

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install maturin
        run: pip install maturin

      - name: Build wheels
        run: maturin build --release --features python --out dist

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: wheels-${{ matrix.os }}
          path: dist/*.whl
```

## QGIS プラグインでの使用例

```python
from qgis.core import QgsRasterLayer, QgsProject
import numpy as np
from osgeo import gdal, osr
import tempfile
import os

# japan_dem をインポート（方法1または2に従って）
import japan_dem

class DEMProcessor:
    def process_dem_xml(self, xml_path, output_path=None):
        """DEM XML を処理して QGIS レイヤーとして追加"""

        # XML をパース
        dem_tile = japan_dem.parse_dem_xml(xml_path)

        # 一時ファイルまたは指定されたパスに出力
        if output_path is None:
            fd, output_path = tempfile.mkstemp(suffix='.tif')
            os.close(fd)

        # GeoTIFF として保存
        self._save_as_geotiff(dem_tile, output_path)

        # QGIS レイヤーとして追加
        layer_name = f"DEM_{dem_tile.metadata.mesh_code}"
        layer = QgsRasterLayer(output_path, layer_name)

        if layer.isValid():
            QgsProject.instance().addMapLayer(layer)
            return layer
        else:
            raise RuntimeError("Failed to create raster layer")

    def _save_as_geotiff(self, dem_tile, output_path):
        """DemTile を GeoTIFF として保存"""
        driver = gdal.GetDriverByName('GTiff')

        # データセットを作成
        ds = driver.Create(
            output_path,
            dem_tile.cols,
            dem_tile.rows,
            1,
            gdal.GDT_Float32
        )

        # 地理変換パラメータを設定
        geotransform = [
            dem_tile.origin_lon,
            dem_tile.x_res,
            0,
            dem_tile.origin_lat + (dem_tile.rows * abs(dem_tile.y_res)),
            0,
            -abs(dem_tile.y_res)
        ]
        ds.SetGeoTransform(geotransform)

        # 投影法を設定 (JGD2011 / EPSG:6668)
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(6668)
        ds.SetProjection(srs.ExportToWkt())

        # データを書き込み
        band = ds.GetRasterBand(1)
        data = np.full((dem_tile.rows, dem_tile.cols), -9999.0, dtype=np.float32)

        # start_point を考慮してデータを配置
        start_x, start_y = dem_tile.start_point
        idx = 0
        for row in range(start_y, dem_tile.rows):
            for col in range(start_x if row == start_y else 0, dem_tile.cols):
                if idx < len(dem_tile.values):
                    data[row, col] = dem_tile.values[idx]
                    idx += 1

        band.WriteArray(data)
        band.SetNoDataValue(-9999.0)
        band.FlushCache()

        # クリーンアップ
        ds = None
```

## トラブルシューティング

### よくある問題

1. **ImportError: DLL load failed**
   - Visual C++ 再頒布可能パッケージがインストールされているか確認
   - Python のバージョンが一致しているか確認

2. **Symbol not found (macOS)**
   - システムの Python バージョンと QGIS の Python バージョンが一致しているか確認
   - 必要に応じて universal2 ビルドを使用

3. **GLIBC version error (Linux)**
   - manylinux2014 ではなく manylinux2010 でビルドを試す
   - または対象システムで直接ビルド

### デバッグ方法

```python
# QGIS Python コンソールで実行
import sys
print(f"Python version: {sys.version}")
print(f"Python executable: {sys.executable}")

# インストール済みパッケージの確認
import subprocess
result = subprocess.run([sys.executable, '-m', 'pip', 'list'], capture_output=True, text=True)
print(result.stdout)
```

## まとめ

QGISプラグインでの配布には、ビルド済みwheelファイルを同梱する「方法1」が最も確実です。これにより：

- ユーザーがインターネット接続なしでインストール可能
- プラットフォーム別の適切なバイナリを自動選択
- QGIS の Python 環境に依存関係なくインストール可能

開発時は GitHub Actions を使って各プラットフォーム用のwheelを自動ビルドし、リリース時にプラグインに含めることを推奨します。
