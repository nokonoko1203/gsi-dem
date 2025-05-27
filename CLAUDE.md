# Rust DEM XML ➜ GeoTIFF コンバータ — プロジェクト設計メモ

## 1. 目的とゴール

* 基盤地図情報 DEM XML（JPGIS‑GML 2014／FGD 5.1）を **ストリーム解析**し、解像度・範囲・CRS を保持した GeoTIFF (Float32) に変換する Rust 製 CLI を作成する。
* DEM5A/B/C・DEM10A/B・DEM1A に対応。
* 4次メッシュ単位の XML → 1タイル GeoTIFF。複数 XML をまとめてディレクトリ一括変換可能。
* GDAL 対応ツールと互換の GeoTIFF を生成（JGD2011/EPSG:6668、NoData=-9999）。

## 2. 全体アーキテクチャ

```text
+---------+        +-----------+        +--------------+
|  CLI    | -----> |  Parser   | -----> | GeoTIFFWriter |
+---------+        +-----------+        +--------------+
      |                  |                       |
      |  Rayon マルチスレッド  |                       |
      +------------------------------------------------+
```

* **cli**: `clap` で引数を解析（入力 XML or 目录, 出力先, スレッド数, 圧縮設定）。
* **parser**: `quick-xml` で SAX 風に読み込み → `DemTile` 構造体へ（行列サイズ / origin / offset / 標高 Vec<f32>）。
* **writer**: `gdal` crate で GTiff ドライバを呼び出し、ジオトランスフォーム・CRS・NoData を設定。
* **error**: `thiserror` + `anyhow` で統一。

### 主要クレート

| 用途               | クレート                        | バージョン目安 |
| ------------------ | ------------------------------- | -------------- |
| CLI                | `clap`                          | 4.x            |
| XML ストリーム解析 | `quick-xml`                     | 0.31           |
| 標高データ格納     | `ndarray` (optional)            | 0.15           |
| GeoTIFF 出力       | `gdal`                          | 0.15           |
| 並列化             | `rayon`                         | 1.8            |
| エラー処理         | `anyhow`, `thiserror`           | 1.0, 1.0       |
| ログ               | `tracing`, `tracing-subscriber` | 0.1            |

### ファイル構成（案）

```text
src/
├── main.rs           # CLI エントリ
├── cli.rs            # 引数定義 & パース
├── lib.rs            # 公開 API
├── parser/
│   └── mod.rs        # XML → DemTile
├── writer/
│   └── mod.rs        # DemTile → GeoTIFF
├── model.rs          # DemTile, Metadata 構造体
├── error.rs          # Error enum
└── utils.rs          # 汎用ヘルパ
examples/
    └── convert_one.rs
```

## 3. 詳細タスクチェックリスト

### A. リポジトリ初期化

* [x] A‑01: GitHub で空の private リポジトリを作成 `dem2tiff-rs`
* [x] A‑02: `cargo init --vcs git` でローカル雛形を生成
* [x] A‑03: `.gitignore` に `target/`, `*.tif`, `*.xml`, `.idea/` を追加
* [x] A‑04: `README.md` に概要／ビルド手順の骨子を記述
* [ ] A‑05: `cargo set-version 0.1.0` で初期バージョン設定

### B. 依存クレート導入

* [x] B‑01: `clap = { version = "4", features = ["derive"] }` を追加
* [x] B‑02: `quick-xml = "0.31"` を追加
* [x] B‑03: `gdal = { version = "0.15", features = ["bindings"] }` を追加
* [x] B‑04: `rayon = "1"` を追加
* [x] B‑05: `anyhow = "1"`, `thiserror = "1"` を追加
* [x] B‑06: `tracing`, `tracing-subscriber` を追加（ログ）

### C. CLI 実装

* [x] C‑01: `cli.rs` に `Args` 構造体を定義（入力 path, 出力 dir, threads, compression）
* [x] C‑02: `main.rs` で `clap::Parser` を呼び出す
* [x] C‑03: 入力が **ファイル** なら 1 件処理、**ディレクトリ** なら再帰的に `.xml` を収集
* [x] C‑04: `rayon::ThreadPoolBuilder` で指定スレッド数に設定
* [x] C‑05: 処理進捗を `indicatif` (optional) で表示

### D. データモデル定義

* [x] D‑01: `DemTile` struct（rows, cols, origin\_lon, origin\_lat, x\_res, y\_res, values: Vec<f32>, startPoint: (usize, usize)）
* [x] D‑02: `Metadata` struct（meshcode, dem\_type, crs\_identifier: String）
* [x] D‑03: `impl DemTile::shape() -> (usize, usize)` など helper を追加

### E. XML パーサ

* [x] E‑01: `parser/mod.rs` 新規作成
* [x] E‑02: `Reader` を `quick_xml::Reader` で初期化（`trim_text = true`）
* [x] E‑03: gml 名前空間を無視してローカル名で分岐
* [x] E‑04: `<mesh>` のテキストを捕捉（実際のタグ名は mesh）
* [x] E‑05: `<gml:GridEnvelope>` から `<gml:high>` 値を取得 → 行列サイズへ変換（+1）
* [x] E‑06: `<gml:Envelope>` から `<lowerCorner>` と `<upperCorner>` を読み取り、緯度経度範囲を取得
* [x] E‑07: Envelope範囲と行列サイズから解像度（x\_res, y\_res）を計算
* [x] E‑08: `<gml:tupleList>` 読み取りを **BufRead** でストリーム処理（カンマ区切りの各行で1要素目を無視、2要素目のみ使用）
* [x] E‑09: 各行を `split(',')`, 2 要素目を `f32` へパース → `values.push()`
* [x] E‑10: startPointを考慮して値の数を検証（行数 \* 列数 - start_x == values.len()）
* [x] E‑11: `<gml:coverageFunction>` → `<gml:GridFunction>` → `<gml:startPoint>` を読み取り
* [x] E‑12: `<gml:Envelope>` の `srsName` 属性から座標系識別子（例: `fguuid:jgd2011.bl`）を読み取る
* [x] E‑13: 解析結果を `DemTile` インスタンスで返却

### F. GeoTIFF ライタ

* [x] F‑01: `writer/mod.rs` 新規作成
* [x] F‑02: `Dataset::create_with_band_type::<f32>` を使い、行列サイズでデータセット生成
* [x] F‑03: `set_geo_transform` で `[origin_lon, x_res, 0, origin_lat, 0, y_res]` を設定（y\_res は負）
* [x] F‑04: `set_projection` に XMLから読み取った座標系識別子に対応するWKTを設定（fguuid:jgd2011.bl → EPSG:6668など）
* [x] F‑05: バンド #1 を取得し、`write()` で `values` を一括書込
* [x] F‑06: NoData 値 -9999 を設定
* [x] F‑07: Deflate 圧縮／TILING オプションを受け取り、`create_with_option` で渡す

### G. Executor

* [x] G‑01: `process_file(path)` → DemTile → GeoTIFF writer → result path を返却
* [x] G‑02: Rayon `par_iter()` でファイルリストを並列処理
* [x] G‑03: 失敗時は `anyhow::Error` を収集し、最後にまとめて表示

### H. テスト

* [x] H‑01: `tests/parser_small.rs` に 2×2 行列の最小 XML サンプルを埋込
* [x] H‑02: `cargo test` が通ることを確認
* [x] H‑03: `tests/roundtrip.rs` で XML → TIFF → GDAL で読み返し、値一致を assert（`gdal::raster::RasterBand::read_as::<f32>`）

### I. CI/CD

* [ ] I‑01: `.github/workflows/ci.yml` で Ubuntu‐latest matrix (stable, beta) で `cargo fmt -- --check`
* [ ] I‑02: `cargo clippy -- -D warnings` を追加
* [ ] I‑03: `cargo test --release` を実行

### J. ドキュメント & 例

* [ ] J‑01: `examples/convert_one.rs` に最小例を実装
* [ ] J‑02: `README.md` に **Usage** セクション（install, convert examples）を追加
* [ ] J‑03: CHANGELOG.md を作成し、v0.1.0 エントリを追加

### K. リリース作業

* [ ] K‑01: `cargo build --release` でバイナリ確認
* [ ] K‑02: GitHub Release Drafter テンプレートを設定
* [ ] K‑03: `cargo publish` (optional) に向けて `categories`・`keywords`・`license` を追記
