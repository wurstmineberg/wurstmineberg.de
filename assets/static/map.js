var map = L.map('map', {
    crs: L.CRS.Simple,
})
.setView([8192, -4096], 0);
L.tileLayer('https://map.wurstmineberg.de/r.{x}.{y}.png', {
    tileSize: 512,
    detectRetina: true,
    minZoom: -4,
    maxZoom: 4,
    minNativeZoom: 0,
    maxNativeZoom: 0,
}).addTo(map);
