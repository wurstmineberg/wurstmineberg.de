function get_user_name() {
    var user;
    var url = document.URL;
    var username = url.substring(url.lastIndexOf("/") + 1, url.length).toLowerCase();
    return username;
}

function display_user_data(person) {
    $('.loading').removeClass('loading');
    
    var name = 'name' in person ? person['name'] : person['id'];
    var ava;
    var head;
    
    $('#username').removeClass('hidden');
    $('#username').text(name);


    if ('minecraft' in person) {
        var minecraft = person['minecraft'];

        ava = '/assets/img/ava/' + minecraft + '.png';
        $('#avatar').attr('src', ava);
        $('#avatar').removeClass('hidden');
        
        head = 'https://minotar.net/avatar/' + minecraft;
        $('#head').attr('src', head);
        $('#head').attr('title', minecraft);
        $('#head').removeClass('hidden');

        if (minecraft.toLowerCase() !== name.toLowerCase()) {
            $('#username').html(name + ' <span class="muted"> (Minecraft: ' + minecraft + ')</span>');
        };
    }
    
    var description = person['description']
    if (!description) {
        description = 'Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)';
        $('#user-description').addClass('muted');
    }
    
    $('#user-description').html(description);
    
    if ('fav_item' in person) {
        var fav_item = person['fav_item'];
        if ('wurstmineberg_image_32x32' in fav_item) {
            $('#fav-item').append('<img src="' + fav_item['wurstmineberg_image_32x32'] + '" /> ');
        }
        if ('wurstmineberg_display_name' in fav_item || 'id' in fav_item) {
            $('#fav-item').removeClass('hidden');
            $('#fav-item').append('wurstmineberg_display_name' in fav_item ? fav_item['wurstmineberg_display_name'] : fav_item['id']);
        }
    }
    
    var social_links = $('#social-links');
    if ('reddit' in person) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + reddit_user_link(person['reddit']) + '">Reddit</a>');
    }

    if ('twitter' in person) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + twitter_user_link(person['twitter']) + '">Twitter</a>');
    }

    if ('website' in person) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + person['website'] + '">Website</a>');
    }
    
    if ('wiki' in person) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + wiki_user_link(person['wiki']) + '">Wiki</a>');
    }
}

function display_stat_data(data) {
	var loading_stat = $('.loading-stat');
	$.each(data, function(stat, value) {

		loading_stat.before('<tr id="' + value + '"><td>' + stat + '</td><td>' + value + '</td></tr>');
	});

	loading_stat.remove();
}

function load_stat_data(minecraft) {
	$.ajax('/assets/world/stats/' + minecraft + '.json', {
		dataType: 'json',
		error: function(request, status, error) {
			$('.loading-stat').html('<td colspan="7">Error: Could not load ' + minecraft + '.json</td>');
		},
		success: function(data) {
			display_stat_data(data);
		}
	});
}

function load_user_data() {
    $.ajax('/assets/serverstatus/people.json', {
        dataType: 'json',
        error: function(request, status, error) {
            $('.loading').html('Error: could not load people.json');
        },
        success: function(data) {
            var username = get_user_name();
            
            if (username != "") {
                data.forEach(function(person) {
                    if ('id' in person) {
                        if (person['id'].toLowerCase() === username) {
                        	if ('minecraft' in person) {
                        		load_stat_data(person['minecraft']);
                        	};

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
