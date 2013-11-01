$.ajax('/assets/serverstatus/people.json', {
    dataType: 'json',
    error: function(request, status, error) {
        $('.loading').children('td').html('error: could not load people.json');
    },
    success: function(data) {
        data.forEach(function(person) {
            var personStatus = 'status' in person ? person['status'] : 'later';

            var name;
            if ('name' in person) {
                name = '<a href="/people/' + person['id'].toLowerCase() + '">' + person['name'] + '</a>';
            } else {
                name = '<a href="/people/' + person['id'].toLowerCase() + '">' + person['id'] + '</a>';
            };
            
            var description = 'description' in person ? '<td class="description">' + person['description'] + '</td>' : '<td class="description small muted">Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)</td>' ;
            $('#loading-' + personStatus + "-table").before('<tr id="' + person['id'] + '"><td class="avatar">&nbsp;</td><td class="username">' + name + '</td>' + description + '</tr>');
        });

        $('.loading').remove();
        data.forEach(function(person) {
            if ('minecraft' in person) {
                var ava = '/assets/img/ava/' + person['minecraft'] + '.png';
                $('#' + person['id'] + ' > .avatar').html('<img class="" />');
                $('#' + person['id'] + ' > .avatar > img').attr('src', ava).error(function() {
                    $('#' + person['id'] + ' > .avatar').html('&nbsp;');
                });
            }
        });
    }
});
