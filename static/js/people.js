$.ajax('/assets/serverstatus/people.json', {
    dataType: 'json',
    error: function(request, status, error) {
        $('.loading').children('td').html('error: could not load people.json');
    },
    success: function(data) {
        data.forEach(function(person) {
            var personStatus = 'status' in person ? person['status'] : 'later';
            var name = 'name' in person ? person['name'] : person['id'];
            var minecraft = 'minecraft' in person ? person['minecraft'] : null;
            var description = 'description' in person ? '<td>' + person['description'] + '</td>' : '<td style="font-size: small; color: gray;">Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)</td>' ;
            var twitter = 'twitter' in person ? '<a href="https://twitter.com/' + person['twitter'] + '">@' + person['twitter'] + '</a>' : '—';
            var reddit = 'reddit' in person ? '<a href="https://reddit.com/u/' + person['reddit'] + '">' + person['reddit'] + '</a>' : '—';
            var website = 'website' in person ? '<a href="' + person['website'] + '">' + url_domain(person['website']) + '</a>' : '—';
            $('#loading-' + personStatus + "-table").before('<tr id="' + person['id'] + '"><td class="avatar">&nbsp;</td><td class="text-info">' + name + '</td>' + description + '<td>' + (minecraft ? minecraft : '—') + '</td><td>' + twitter + '</td><td>' + reddit + '</td><td>' + website + '</td></tr>');
        });

        $('.loading').remove();
        data.forEach(function(person) {
            if ('minecraft' in person) {
                var ava = '/assets/img/ava/' + person['minecraft'] + '.png';
                $('#' + person['id'] + ' > .avatar').html('<img class="img-rounded" alt="avatar" />');
                $('#' + person['id'] + ' > .avatar > img').attr('src', ava).error(function() {
                    $('#' + person['id'] + ' > .avatar').html('&nbsp;');
                });
            }
        });
    }
});
