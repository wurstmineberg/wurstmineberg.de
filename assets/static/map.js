var map = L.map('map', {
    crs: L.CRS.Simple,
})
.setView([16384, -8192], 0);
L.tileLayer('https://map.wurstmineberg.de/r.{x}.{y}.png', {
    tileSize: 512,
    detectRetina: false, // breaks the map when zooming in or out too far
    minZoom: -4, // should not be decreased without also decreasing min native zoom
    maxZoom: 4,
    minNativeZoom: 0,
    maxNativeZoom: 0,
}).addTo(map);
