var map = L.map('map', {
    crs: L.CRS.Simple,
})
.setView([16384, -8192], 0);
L.tileLayer('https://map.wurstmineberg.de/r.{x}.{y}.png', {
    tileSize: 512,
    detectRetina: true,
    minZoom: -18,
    maxZoom: 4,
    minNativeZoom: 0,
    maxNativeZoom: 0,
}).addTo(map);
