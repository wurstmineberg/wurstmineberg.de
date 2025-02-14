var map = L.map('map', {
    crs: L.CRS.Simple,
})
.setView([0, 0], 0);
L.tileLayer('https://wurstmineberg.de/api/v3/world/wurstmineberg/map/{z}/{x}/{y}.png', {
    detectRetina: true,
}).addTo(map);
