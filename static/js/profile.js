function bind_pagination_events() {
    $('.pagination-item').bind('click', function(eventObject) {
        var id = $(this).attr('id')
        var elementid = id.substring('pagination-'.length, id.length);
        var selected = $('#' + elementid);
        $('.stats-section').each(function(index, element) {
            var table = $(element);
            if (table.attr('id') == selected.attr('id')) {
                table.removeClass("hidden");
            } else {
                table.addClass("hidden");
            }
        });
    });
}

function get_user_name() {
    var user;
    var url = document.URL;
    var username = url.substring(url.lastIndexOf("/") + 1, url.length).toLowerCase();
    hashindex = username.lastIndexOf("#");
    if (hashindex > 0) {
        username = username.substring(0, username.lastIndexOf("#"));
    };
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

function is_block(id) {
    return false;
}

function display_stat_data(data) {
    var loading_stat_general = $('#loading-stat-general-table');
    var loading_stat_item = $('#loading-stat-items-table');
    var loading_stat_block = $('#loading-stat-blocks-table');
    var loading_stat_general = $('#loading-stat-general-table');
    var loading_stat_mobs = $('#loading-stat-mobs-table');

    $.each(data, function(key, value) {
        stat = key.split('.');
        var name;

        if (stat[0] === 'stat') {
            if (stat[1] === 'craftItem' ||
                stat[1] === 'useItem' ||
                stat[1] === 'breakItem') {
                var item = stat[2];
                name = item;

                var row = $('#item-row-' + item);
                if (row.length == 0) {
                    row = '<tr id="item-row-' + item + '" class="item-row"><td class="name"></td><td class="depleted">0</td><td class="crafted">0</td><td class="used">0</td></tr>';
                    loading_stat_item.before(row);
                    row = $('#item-row-' + item);
                    row.children('.name').text(name);
                }

                if (row) {
                    if (stat[1] === 'craftItem') {
                        row.children('.crafted').text(value);
                    } else if (stat[1] === 'useItem') {
                        row.children('.used').text(value);
                    } else if (stat[1] === 'breakItem') {
                        row.children('.depleted').text(value);
                    }
                }

            } else if (stat[1] === 'mineBlock') {
                var item = stat[2];
                name = item;

                var row = $('#block-row-' + item);
                if (row.length == 0) {
                    row = '<tr id="block-row-' + item + '" class="block-row"><td class="name"></td><td class="crafted">0</td><td class="used">0</td><td class="mined">0</td></tr>';
                    loading_stat_block.before(row);
                    row = $('#block-row-' + item);
                    row.children('.name').text(name);
                }

                if (row) {
                    if (stat[1] === 'mineBlock') {
                        row.children('.mined').text(value);
                    }
                }
            } else if (stat[1] === 'killEntity' ||
                       stat[1] === 'entityKilledBy') {
                var mobname = stat[2];
                var row = $('#mob-row-' + mobname);
                if (row.length == 0) {
                    row = '<tr id="mob-row-' + mobname + '" class="mob-row"><td class="name"></td><td class="killed">0</td><td class="killed-by">0</td></tr>';
                    loading_stat_mobs.before(row);
                    row = $('#mob-row-' + mobname);
                    row.children('.name').text(mobname);
                }

                if (stat[1] === 'killEntity') {
                    row.children('.killed').text(value);
                } else if (stat[1] === 'entityKilledBy') {
                    row.children('.killed-by').text(value);
                }
            } else {
                name = key.substring('stat.'.length, key.length);


                var row = '<tr id="general-row-' + name + '" class="general-row"><td class="name">' + name + '</td><td class="info">' + value + '</td></tr>'
                loading_stat_general.before(row);
            }
        }
    });

    $('.loading-stat').remove();
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

bind_pagination_events();
load_user_data();
