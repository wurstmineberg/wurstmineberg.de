document.querySelectorAll('.date').forEach(function(date) {
    date.textContent = new Date(parseInt(date.dataset.timestamp)).toLocaleString([], {dateStyle: 'medium'});
});

document.querySelectorAll('.datetime').forEach(function(dateTime) {
    var longFormat = dateTime.dataset.long == 'true';
    dateTime.textContent = new Date(parseInt(dateTime.dataset.timestamp)).toLocaleString([], {
        dateStyle: longFormat ? 'full' : 'medium',
        timeStyle: longFormat ? 'full' : 'short',
    });
});
