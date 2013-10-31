function get_user_name() {
    var user;
    var url = document.URL;
    var username = url.substring(url.lastIndexOf("/") + 1, url.length).toLowerCase();
    return username;
}

function display_user_data(person) {
    $('.loading').removeClass('loading');

    var ava;
    var head;

    if ('minecraft' in person) {
        ava = '/assets/img/ava/' + person['minecraft'] + '.png';
        $('#avatar').attr('src', ava);
        $('#avatar').removeClass('hidden');

        head = 'https://minotar.net/avatar/' + person['minecraft'];
        $('#head').attr('src', head);
        $('#head').removeClass('hidden');
    }

    $('#username').removeClass('hidden');
    $('#username').text(person['name']);

    var description = person['description']
    if (!description) {
        description = 'Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)';
        $('#user-description').addClass('muted');
    };

    $('#user-description').html(description);
}

function load_user_data() {
    $.ajax('/assets/serverstatus/people.json', {
        dataType: 'json',
        error: function(request, status, error) {
            $('.loading').html('Error: could not load people.json');
        },
        success: function(data) {
            var username = get_user_name();
            var user;

            if (username != "") {
                data.forEach(function(person) {
                    if ('id' in person) {
                        if (person['id'].toLowerCase() === username) {
                            display_user_data(person);
                            return;
                        }
                    }
                });


                $('.loading').html('Error: User with this name not found');
            }
        }
    });
}

load_user_data();
