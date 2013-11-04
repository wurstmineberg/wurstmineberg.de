$.ajax('/assets/serverstatus/people.json', {
    dataType: 'json',
    error: function(request, status, error) {
        $('.loading').children('td').html('error: could not load people.json');
    },
    success: function(data) {
        var people = new People(data);

        people.list.forEach(function(person) {
        	var minecraft = '';

            if (person.minecraft && person.minecraft.toLowerCase() !== person.interfaceName.toLowerCase()) {
                minecraft = '<p class="muted">' + person['minecraft'] + '</p>'
            };

            name = '<a href="/people/' + person.id + '">' + person.interfaceName + '</a>' + minecraft;
            
            var description;
            if (!person.description) {
            	description = '<td class="description small muted">Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)</td>'
            } else {
            	description = '<td class="description">' + person['description'] + '</td>';
            }

            $('#loading-' + person.status + "-table").before('<tr id="' + person.id + '"><td class="avatar">&nbsp;</td><td class="username">' + name + '</td>' + description + '</tr>');

            if (person.minecraft) {
            	var ava = '/assets/img/ava/' + person.minecraft + '.png';
                $('#' + person.id + ' > .avatar').html('<img class="" />');
                $('#' + person.id + ' > .avatar > img').attr('src', ava).error(function() {
                	$('#' + person.id + ' > .avatar').html('&nbsp;');
                });
            };
        });

        $('.loading').remove();
    }
});
