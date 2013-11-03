function bind_tab_events() {
    $('.tab-item').bind('click', function(eventObject) {
        eventObject.preventDefault();
        $(this).tab('show');
    });

    $('.tab-item').on('show.bs.tab', function(e) {
        var id = $(this).attr('id')
        var elementid = id.substring('tab-'.length, id.length);
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

    $("#tab-stats-general").tab('show');

    if (location.hash !== '') $('a[href="' + location.hash + '"]').tab('show');
        return $('a.tab-item').on('shown.bs.tab', function(e) {
            return location.hash = $(e.target).attr('href').substr(1);
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
    var loading_stat_achievements = $('#loading-stat-achievements-table');

    var general = [];
    var items = [];
    var blocks = [];
    var mobs = [];
    var achievements = [];

    // wait for the string data to arrive
    $.when(fetch_string_data(), fetch_item_data()).done(function(string_data, item_data) {
        string_data = string_data[0];
        item_data = item_data[0];
        $.each(data, function(key, value) {
            stat = key.split('.');
            var name;

            if (stat[0] === 'stat') {
                if (stat[1] === 'craftItem' ||
                    stat[1] === 'useItem' ||
                    stat[1] === 'breakItem' ||
                    stat[1] === 'mineBlock') {
                    var id = parseInt(stat[2]);
                    var name = stat[2];
                    var actionIndex = stat[1];
                    var count = value;

                    var collection;
                    if (id >= 256) {
                        collection = items;
                    } else {
                        collection = blocks;
                    }

                    var info;
                    if ('' + id in item_data) {
                        info = item_data['' + id];
                        name = info['name'];
                    }

                    var found = false;
                    $.each(collection, function(key, value) {
                        if (value['id'] === id) {
                            value[actionIndex] = count;
                            found = true;
                            return;
                        }
                    });

                    if (!found) {
                        newEntry = {'name': name, 'id': id};
                        newEntry[actionIndex] = count;
                        if (info) {
                            newEntry['info'] = info;
                        };
                        collection.push(newEntry);
                    }

                } else if (stat[1] === 'killEntity' ||
                           stat[1] === 'entityKilledBy') {
                    var id = stat[2];
                    var actionIndex = stat[1];
                    var count = value;

                    var name = id;
                    if ('stats' in string_data) {
                        if ('mobs' in string_data['stats']) {
                            if (stat[2] in string_data['stats']['mobs']) {
                                name = string_data['stats']['mobs'][stat[2]];
                            };
                        };
                    };

                    var found = false;
                    $.each(mobs, function(key, value) {
                        if (value['id'] === id) {
                            value[actionIndex] = count;
                            found = true;
                            return;
                        }
                    });

                    if (!found) {
                        newEntry = {'id': id, 'name': name};
                        newEntry[actionIndex] = count;
                        mobs.push(newEntry);
                    };

                } else {
                    var final_key = key;
                    var final_value = prettify_stats_value(key, value);

                    if ('stats' in string_data) {
                        if ('general' in string_data['stats']) {
                            if (stat[1] in string_data['stats']['general']) {
                                final_key = string_data['stats']['general'][stat[1]];
                            };
                        };
                    };

                    general.push({'name': final_key, 'value': final_value});
                }
            } else {
                if (stat[0] === 'achievement') {
                    var id = stat[1];
                    var name = id;
                    var description = "";

                    if ('stats' in string_data) {
                        if ('achievements' in string_data['stats']) {
                            if (id in string_data['stats']['achievements']) {
                                name = string_data['stats']['achievements'][id][0];
                                description = string_data['stats']['achievements'][id][1];
                            };
                        };
                    };

                    var final_value = value;
                    if (stat[1] === 'exploreAllBiomes') {
                        if ('value' in value) {
                            if (value['value'] === 1) {
                                final_value = "Yes"
                            } else {
                                if ('progress' in value) {
                                    final_value = 'Progress: ';
                                    $.each(value['progress'], function(index, biome) {
                                        final_value += biome + ', ';
                                    });
                                    final_value = final_value.substring(0, final_value.length - 2);
                                }
                            }
                        }
                    } else {
                        if (parseInt(value) >= 1) {
                            final_value = 'Yes';
                        } else {
                            final_value = 'No'
                        }
                    }

                    achievements.push({'id': id, 'name': name, 'description': description, 'value': final_value});
                };
            }
        });

        // Add the missing achievements
        if ('stats' in string_data) {
            if ('achievements' in string_data['stats']) {
                $.each(string_data['stats']['achievements'], function(id, stringarray) {
                    var alreadyExisting = false;
                    $.each(achievements, function(index, dict) {
                        if (id === dict['id']) {
                            alreadyExisting = true;
                            return;
                        };
                    });

                    if (!alreadyExisting) {
                        achievements.push({'name': stringarray[0], 'description': stringarray[1], 'value': 'No'});
                    };
                });
            };
        };

        general.sort(function(a, b) {
            nameA = a['name'];
            nameB = b['name'];
            return nameA.localeCompare(nameB);
        });

        mobs.sort(function(a, b) {
            nameA = a['name'];
            nameB = b['name'];
            return nameA.localeCompare(nameB);
        });

        items.sort(function(a, b) {
            return a['id'] - b['id'];
        });

        blocks.sort(function(a, b) {
            return a['id'] - b['id'];
        });

        achievements.sort(function(a, b) {
            nameA = a['name'];
            nameB = b['name'];
            return nameA.localeCompare(nameB);
        });


        $.each(general, function(index, dict) {
            name = dict['name'];
            value = dict['value'];

            var row = '<tr id="general-row-' + name + '" class="general-row"><td class="name">' + name + '</td><td class="info">' + value + '</td></tr>'
            loading_stat_general.before(row);
        });

        $.each(mobs, function(index, dict) {
            name = dict['name'];
            id = dict['id']

            row = '<tr id="mob-row-' + id + '" class="mob-row"><td class="name"></td><td class="killed">0</td><td class="killed-by">0</td></tr>';
            loading_stat_mobs.before(row);
            row = $('#mob-row-' + id);
            row.children('.name').text(name);

            if ('killEntity' in dict) {
                row.children('.killed').text(dict['killEntity']);
            }

            if ('entityKilledBy' in dict) {
                row.children('.killed-by').text(dict['entityKilledBy']);
            }
        });

        $.each(items, function(index, dict) {
            var name = dict['name'];
            var id = dict['id'];
            var image = "";
            if ('info' in dict) {
                var info = dict['info'];
                var image = '<img src="' + info['image'] + '" alt="image" class="item-image" />';
            }

            var row = '<tr id="item-row-' + id + '" class="item-row"><td class="image"></td><td class="name"></td><td class="depleted">0</td><td class="crafted">0</td><td class="used">0</td></tr>';
            loading_stat_item.before(row);
            row = $('#item-row-' + id);
            row.children('.name').text(name);
            row.children('.image').html(image);

            if ('craftItem' in dict) {
                row.children('.crafted').text(dict['craftItem']);
            }

            if ('useItem' in dict) {
                row.children('.used').text(dict['useItem']);
            }

            if ('breakItem' in dict) {
                row.children('.depleted').text(dict['breakItem']);
            }
        });

        $.each(blocks, function(index, dict) {
            var name = dict['name'];
            var id = dict['id'];
            var image = "";
            if ('info' in dict) {
                var info = dict['info'];
                var image = '<img src="' + info['image'] + '" alt="image" class="item-image" />';
            }

            var row = '<tr id="block-row-' + id + '" class="block-row"><td class="image"></td><td class="name"></td><td class="crafted">0</td><td class="used">0</td><td class="mined">0</td></tr>';
            loading_stat_block.before(row);
            row = $('#block-row-' + id);
            row.children('.name').text(name);
            row.children('.image').html(image);

            if ('craftItem' in dict) {
                row.children('.crafted').text(dict['craftItem']);
            }

            if ('useItem' in dict) {
                row.children('.used').text(dict['useItem']);
            }

            if ('mineBlock' in dict) {
                row.children('.mined').text(dict['mineBlock']);
            }
        });

        $.each(achievements, function(index, dict) {
            name = dict['name'];
            description = dict['description'];
            value = dict['value']

            if (value === "Yes") {
                value = '<span class="glyphicon glyphicon-ok"></span>'
            } else if (value === "No") {
                value = '<span class="glyphicon glyphicon-remove"></span>'
            }

            row = '<tr id="achievement-row-' + name + '" class="achievement-row"><td class="name"><a href="#" data-toggle="tooltip" data-placement="right" rel="tooltip" class="text-link" title="' + description + '">' + name + '</a></td><td class="value">' + value + '</td></tr>';
            loading_stat_achievements.before(row);
        });

        $('.loading-stat').remove();
        initialize_tooltips();
    });
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

bind_tab_events();
load_user_data();
