"""
QGIS Plugin Example for GSI DEM Parser

This example demonstrates how to use the gsi_dem Python bindings 
in a QGIS plugin to parse GSI DEM XML files and create raster layers.
"""

import os
import numpy as np
from qgis.core import (
    QgsRasterLayer,
    QgsRasterDataProvider,
    QgsRectangle,
    QgsCoordinateReferenceSystem,
    QgsProject,
    QgsRasterBandStats
)
from qgis.PyQt.QtWidgets import QAction, QFileDialog, QMessageBox
from qgis.PyQt.QtCore import Qt

# Import the gsi_dem module
import gsi_dem


class GsiDemLoaderPlugin:
    """QGIS Plugin for loading GSI DEM XML files"""
    
    def __init__(self, iface):
        self.iface = iface
        self.plugin_dir = os.path.dirname(__file__)
        
    def initGui(self):
        """Create the menu entries and toolbar icons inside the QGIS GUI"""
        # Create action
        self.action = QAction("Load GSI DEM XML", self.iface.mainWindow())
        self.action.triggered.connect(self.run)
        
        # Add toolbar button and menu item
        self.iface.addToolBarIcon(self.action)
        self.iface.addPluginToRasterMenu("GSI DEM Loader", self.action)
        
    def unload(self):
        """Remove the plugin menu item and icon"""
        self.iface.removePluginRasterMenu("GSI DEM Loader", self.action)
        self.iface.removeToolBarIcon(self.action)
        
    def run(self):
        """Run method that loads the GSI DEM XML file"""
        # Get file path from user
        file_path, _ = QFileDialog.getOpenFileName(
            self.iface.mainWindow(),
            "Select GSI DEM XML file",
            "",
            "XML files (*.xml)"
        )
        
        if not file_path:
            return
            
        try:
            # Parse the DEM XML file using gsi_dem
            dem_tile = gsi_dem.parse_dem_xml(file_path)
            
            # Create a temporary GeoTIFF file
            import tempfile
            temp_dir = tempfile.gettempdir()
            output_path = os.path.join(
                temp_dir, 
                f"dem_{dem_tile.metadata.mesh_code}.tif"
            )
            
            # Convert to GeoTIFF using GDAL
            self._create_geotiff(dem_tile, output_path)
            
            # Load as QGIS raster layer
            layer_name = f"DEM {dem_tile.metadata.mesh_code}"
            raster_layer = QgsRasterLayer(output_path, layer_name)
            
            if raster_layer.isValid():
                # Add layer to project
                QgsProject.instance().addMapLayer(raster_layer)
                
                # Zoom to layer extent
                self.iface.mapCanvas().setExtent(raster_layer.extent())
                self.iface.mapCanvas().refresh()
                
                # Show success message
                QMessageBox.information(
                    self.iface.mainWindow(),
                    "Success",
                    f"Successfully loaded DEM {dem_tile.metadata.mesh_code}\n"
                    f"Size: {dem_tile.cols} x {dem_tile.rows}\n"
                    f"Resolution: {dem_tile.x_res:.6f} x {dem_tile.y_res:.6f}"
                )
            else:
                QMessageBox.critical(
                    self.iface.mainWindow(),
                    "Error",
                    "Failed to create valid raster layer"
                )
                
        except Exception as e:
            QMessageBox.critical(
                self.iface.mainWindow(),
                "Error",
                f"Failed to load DEM: {str(e)}"
            )
    
    def _create_geotiff(self, dem_tile, output_path):
        """Create a GeoTIFF file from DemTile data"""
        try:
            from osgeo import gdal, osr
            
            # Create driver
            driver = gdal.GetDriverByName('GTiff')
            
            # Create dataset
            dataset = driver.Create(
                output_path,
                dem_tile.cols,
                dem_tile.rows,
                1,  # Number of bands
                gdal.GDT_Float32
            )
            
            # Set geotransform
            # GDAL geotransform: [origin_x, pixel_width, 0, origin_y, 0, -pixel_height]
            geotransform = [
                dem_tile.origin_lon,
                dem_tile.x_res,
                0,
                dem_tile.origin_lat,
                0,
                -dem_tile.y_res  # Negative because origin is at top-left
            ]
            dataset.SetGeoTransform(geotransform)
            
            # Set projection
            srs = osr.SpatialReference()
            if "jgd2011" in dem_tile.metadata.crs_identifier.lower():
                srs.ImportFromEPSG(6668)  # JGD2011 geographic
            else:
                srs.ImportFromEPSG(4326)  # Default to WGS84
            dataset.SetProjection(srs.ExportToWkt())
            
            # Write data
            band = dataset.GetRasterBand(1)
            
            # Create full array with NoData values
            data_array = np.full((dem_tile.rows, dem_tile.cols), -9999.0, dtype=np.float32)
            
            # Fill with actual values starting from start_point
            start_x, start_y = dem_tile.start_point
            value_idx = 0
            for row in range(start_y, dem_tile.rows):
                for col in range(start_x if row == start_y else 0, dem_tile.cols):
                    if value_idx < len(dem_tile.values):
                        data_array[row, col] = dem_tile.values[value_idx]
                        value_idx += 1
            
            band.WriteArray(data_array)
            band.SetNoDataValue(-9999.0)
            
            # Close dataset
            dataset = None
            
        except ImportError:
            raise RuntimeError("GDAL Python bindings not found. Please install python-gdal.")


# Example usage in QGIS Python console:
"""
# After installing the gsi_dem module with maturin:
# pip install maturin
# maturin develop --release

# In QGIS Python console:
import gsi_dem

# Parse a DEM XML file
dem_tile = gsi_dem.parse_dem_xml('/path/to/your/dem.xml')

# Access properties
print(f"Mesh code: {dem_tile.metadata.mesh_code}")
print(f"Size: {dem_tile.cols} x {dem_tile.rows}")
print(f"Origin: ({dem_tile.origin_lon}, {dem_tile.origin_lat})")
print(f"Resolution: {dem_tile.x_res} x {dem_tile.y_res}")
print(f"Number of values: {len(dem_tile.values)}")
print(f"CRS: {dem_tile.metadata.crs_identifier}")
"""