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

function initialize_datatables() {
    /* Set the defaults for DataTables initialisation */
    var table = $('#stats-blocks-table').dataTable({
        "bPaginate": false,
        "bAutoWidth": false,
        "bLengthChange": false,
        "bFilter": false,
        "sDom": "<'row-fluid'<'span6'f><'span6'<'pull-right'T>>r>t",
    });
    new FixedHeader(table)
}

function display_user_data(person, item_data) {
    $('.loading').removeClass('loading');
    
    var name = person.interfaceName;
    var ava;
    var head;
    
    $('#username').removeClass('hidden');
    $('#username').text(name);

    if (person.minecraft) {
        var minecraft = person.minecraft;

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
    
    var description = person.description
    if (!description) {
        description = 'Hier könnte Ihre Beschreibung stehen! (To update your description, tell someone in <a href="irc://chat.freenode.net/#wurstmineberg">IRC</a>.)';
        $('#user-description').addClass('muted');
    }
    
    $('#user-description').html(description);
    
    var fav_item = person.fav_item;
    if (fav_item) {
        if ('id' in fav_item) {
            var fav_item_data = fav_item;
            if (fav_item['id'].toString() in item_data) {
                fav_item_data = item_data[fav_item['id'].toString()];
                if ('Damage' in fav_item && (fav_item['id'] + ':' + fav_item['Damage']) in item_data) {
                    fav_item_data = item_data[fav_item['id'] + ':' + fav_item['Damage']];
                }
            }
            $('#fav-item').removeClass('hidden');
            if ('image' in fav_item_data) {
                $('#fav-item').append('<img src="' + fav_item_data['image'] + '" /> ');
            }
            $('#fav-item').append('name' in fav_item_data ? fav_item_data['name'] : fav_item['id']);
        }
    }
    
    var social_links = $('#social-links');
    if (person.reddit) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + reddit_user_link(person.reddit) + '">Reddit</a>');
    }

    if (person.twitter) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + twitter_user_link(person.twitter) + '">Twitter</a>');
    }

    if (person.website) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + person.website + '">Website</a>');
    }
    
    if (person.wiki) {
        social_links.removeClass('hidden');
        social_links.append('<a class="social-link" href="' + wiki_user_link(person['wiki']) + '">Wiki</a>');
    }
}

function is_block(id) {
    return false;
}

function display_stat_data(stat_data, string_data, item_data, achievement_data) {
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

    $.each(stat_data, function(key, value) {
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
                var final_value = prettify_stats_value(stat[1], value);

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
                if (id in achievement_data) {
                    name = achievement_data[id]['displayname'];
                    description = achievement_data[id]['description'];
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
                
                achievements.push({
                    'id': id,
                    'name': name,
                    'description': description,
                    'value': final_value
                });
            };
        }
    });

    // Add the missing achievements
    $.each(achievement_data, function(id, achievement_dict) {
        var alreadyExisting = false;
        $.each(achievements, function(index, dict) {
            if (id === dict['id']) {
                alreadyExisting = true;
                return;
            };
        });

        if (!alreadyExisting) {
            achievements.push({'name': achievement_dict['displayname'], 'description': achievement_dict['description'], 'value': 'No'});
        };
    });

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
            if ('image' in info) {
                var image = '<img src="' + info['image'] + '" alt="image" class="item-image" />';
            };
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
            if ('image' in info) {
                var image = '<img src="' + info['image'] + '" alt="image" class="item-image" />';
            };
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
            value = '<span class="glyphicon glyphicon-ok text-success"></span>'
        } else if (value === "No") {
            value = '<span class="glyphicon glyphicon-remove text-danger"></span>'
        }

        row = '<tr id="achievement-row-' + name + '" class="achievement-row"><td class="name"><a href="#" data-toggle="tooltip" data-placement="right" rel="tooltip" class="text-link" title="' + description + '">' + name + '</a></td><td class="value">' + value + '</td></tr>';
        loading_stat_achievements.before(row);
    });

    $('.loading-stat').remove();
    initialize_tooltips();

    initialize_datatables();
}

function load_stat_data(person, string_data, item_data, achievement_data) {
    $.when(API.personStatData(person))
        .done(function(stat_data) {
            display_stat_data(stat_data, string_data, item_data, achievement_data);
        })
        .fail(function() {
            $('.loading-stat').html('<td colspan="7">Error: Could not load ' + person.minecraft + '.json</td>');
        });
}

function load_user_data() {
    var username = get_user_name();

    $.when(API.personById(username), API.stringData(), API.itemData(), API.achievementData())
        .done(function(person, string_data, item_data, achievement_data) {
            load_stat_data(person, string_data, item_data, achievement_data);
            display_user_data(person, item_data);
        }).fail(function() {
            $('.loading').html('Error: User with this name not found');
        });
}


select_tab_with_id("tab-stats-general");
bind_tab_events();
load_user_data();
