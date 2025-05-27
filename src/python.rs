use crate::model::DemTile;
use crate::parser;
use pyo3::prelude::*;
use std::fs::File;
use std::io::BufReader;

#[pymodule]
fn japan_dem(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDemTile>()?;
    m.add_class::<PyMetadata>()?;
    m.add_function(wrap_pyfunction!(parse_dem_xml, m)?)?;
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
