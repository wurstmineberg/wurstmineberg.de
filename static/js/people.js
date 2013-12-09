function display_people_data(people) {
    people.list.forEach(function(person) {
        if (!person.status in ['founding', 'later', 'postfreeze', 'former']) {
            return;
        }
        var minecraft = '';

        if (person.minecraft && person.minecraft.toLowerCase() !== person.interfaceName.toLowerCase()) {
            minecraft = '<p class="muted">' + person['minecraft'] + '</p>'
        };

        name = '<a href="/people/' + person.id + '">' + person.interfaceName + '</a>' + minecraft;
        
        var description;
        if (!person.description) {
            description = '<td class="description small muted">Hier könnte Ihre Beschreibung stehen! (You can update your description using the command <code>people ' + person.id + ' description</code>.)</td>'
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
};

function load_people_data() {
    $.when(API.people())
        .done(function(people) {
            display_people_data(people);
        })
        .fail(function() {
            $('.loading').children('td').html('Error: could not load people.json');
        });
};

load_people_data();
