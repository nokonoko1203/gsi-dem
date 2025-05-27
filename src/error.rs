use thiserror::Error;

#[derive(Error, Debug)]
pub enum DemError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parse error: {0}")]
    XmlParse(#[from] quick_xml::Error),

    // TODO: GDALエラーは後で追加
    // #[error("GDAL error: {0}")]
    // Gdal(#[from] gdal::errors::GdalError),
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}
