var map = L.map('map', {
    crs: L.CRS.Simple,
})
.setView([0, 0], 0);
L.tileLayer('https://map.wurstmineberg.de/r.{x}.{y}.png', {
    detectRetina: true,
    minZoom: 0,
    maxZoom: 0,
}).addTo(map);
