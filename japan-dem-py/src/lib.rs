use japan_dem::model::DemTile;
use japan_dem::parser;
use japan_dem::terrain_rgb::{elevation_to_rgb, rgb_to_elevation, TerrainRgbConfig};
use japan_dem::writer::GeoTiffWriter;
use pyo3::prelude::*;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[pymodule]
fn japan_dem(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDemTile>()?;
    m.add_class::<PyMetadata>()?;
    m.add_function(wrap_pyfunction!(parse_dem_xml, m)?)?;
    m.add_function(wrap_pyfunction!(dem_to_terrain_rgb, m)?)?;
    m.add_function(wrap_pyfunction!(elevation_to_rgb_py, m)?)?;
    m.add_function(wrap_pyfunction!(rgb_to_elevation_py, m)?)?;
    Ok(())
}

#[pyclass(name = "DemTile")]
#[derive(Clone)]
pub struct PyDemTile {
    #[pyo3(get)]
    pub rows: usize,
    #[pyo3(get)]
    pub cols: usize,
    #[pyo3(get)]
    pub origin_lon: f64,
    #[pyo3(get)]
    pub origin_lat: f64,
    #[pyo3(get)]
    pub x_res: f64,
    #[pyo3(get)]
    pub y_res: f64,
    #[pyo3(get)]
    pub values: Vec<f32>,
    #[pyo3(get)]
    pub start_point: (usize, usize),
    #[pyo3(get)]
    pub metadata: PyMetadata,
}

#[pyclass(name = "Metadata")]
#[derive(Clone)]
pub struct PyMetadata {
    #[pyo3(get)]
    pub mesh_code: String,
    #[pyo3(get)]
    pub dem_type: String,
    #[pyo3(get)]
    pub crs_identifier: String,
}

impl From<DemTile> for PyDemTile {
    fn from(tile: DemTile) -> Self {
        PyDemTile {
            rows: tile.rows,
            cols: tile.cols,
            origin_lon: tile.origin_lon,
            origin_lat: tile.origin_lat,
            x_res: tile.x_res,
            y_res: tile.y_res,
            values: tile.values,
            start_point: tile.start_point,
            metadata: PyMetadata {
                mesh_code: tile.metadata.meshcode,
                dem_type: tile.metadata.dem_type,
                crs_identifier: tile.metadata.crs_identifier,
            },
        }
    }
}

#[pymethods]
impl PyDemTile {
    #[getter]
    fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    fn __repr__(&self) -> String {
        format!(
            "DemTile(rows={}, cols={}, origin=({}, {}), resolution=({}, {}), mesh_code={})",
            self.rows,
            self.cols,
            self.origin_lon,
            self.origin_lat,
            self.x_res,
            self.y_res,
            self.metadata.mesh_code
        )
    }
}

#[pymethods]
impl PyMetadata {
    fn __repr__(&self) -> String {
        format!(
            "Metadata(mesh_code='{}', dem_type='{}', crs='{}')",
            self.mesh_code, self.dem_type, self.crs_identifier
        )
    }
}

#[pyfunction]
pub fn parse_dem_xml(path: &str) -> PyResult<PyDemTile> {
    let file = File::open(path).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to open file: {}", e))
    })?;
    let reader = BufReader::new(file);

    let dem_tile = parser::parse_dem_xml(reader).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to parse XML: {}", e))
    })?;

    Ok(PyDemTile::from(dem_tile))
}

impl From<PyDemTile> for DemTile {
    fn from(py_tile: PyDemTile) -> Self {
        DemTile {
            rows: py_tile.rows,
            cols: py_tile.cols,
            origin_lon: py_tile.origin_lon,
            origin_lat: py_tile.origin_lat,
            x_res: py_tile.x_res,
            y_res: py_tile.y_res,
            values: py_tile.values,
            start_point: py_tile.start_point,
            metadata: japan_dem::model::Metadata {
                meshcode: py_tile.metadata.mesh_code,
                dem_type: py_tile.metadata.dem_type,
                crs_identifier: py_tile.metadata.crs_identifier,
            },
        }
    }
}

#[pyfunction]
#[pyo3(signature = (dem_tile, output_path, min_elevation=None, max_elevation=None))]
pub fn dem_to_terrain_rgb(
    dem_tile: PyDemTile,
    output_path: &str,
    min_elevation: Option<f32>,
    max_elevation: Option<f32>,
) -> PyResult<()> {
    let config = TerrainRgbConfig {
        min_elevation,
        max_elevation,
    };

    let dem_tile: DemTile = dem_tile.into();
    let writer = GeoTiffWriter::new();
    writer
        .write_terrain_rgb(&dem_tile, Path::new(output_path), &config)
        .map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!(
                "Failed to convert to terrain RGB: {}",
                e
            ))
        })
}

#[pyfunction]
pub fn elevation_to_rgb_py(elevation: f32) -> (u8, u8, u8) {
    elevation_to_rgb(elevation)
}

#[pyfunction]
pub fn rgb_to_elevation_py(r: u8, g: u8, b: u8) -> f32 {
    rgb_to_elevation(r, g, b)
}
