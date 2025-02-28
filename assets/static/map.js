var map = L.map('map', {
    crs: L.CRS.Simple,
    minZoom: -4,
    maxZoom: 4,
})
.setView([8192, -4096], 0);
L.tileLayer('https://map.wurstmineberg.de/r.{x}.{y}.png', {
    detectRetina: true,
    minZoom: 0,
    maxZoom: 0,
}).addTo(map);
