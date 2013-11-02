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
    var loading_stat_achievements = $('#loading-stat-achievements-table');

    var general = [];
    var items = [];
    var blocks = [];
    var mobs = [];
    var achievements = [];

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
                    collection.push(newEntry);
                }

            } else if (stat[1] === 'killEntity' ||
                       stat[1] === 'entityKilledBy') {
                var mobname = stat[2];
                var actionIndex = stat[1];
                var count = value;

                var found = false;
                $.each(mobs, function(key, value) {
                    if (value['name'] === mobname) {
                        value[actionIndex] = count;
                        found = true;
                        return;
                    }
                });

                if (!found) {
                	newEntry = {'name': mobname};
                	newEntry[actionIndex] = count;
                    mobs.push(newEntry);
                };

            } else {
            	var final_value = value;
            	if (key.endsWith('OneCm')) {
            		if (value > 1000000) {
            			final_value = (value / 1000000).toFixed(2) + ' km';
            		} else if (value > 1000) {
            			final_value = (value / 1000).toFixed(2) + ' m';
            		} else {
            			final_value = value + ' cm';
            		}
            	} else if (key.endsWith('OneMinute')) {
            		// Yes, this is called 'minute' and actually reflects the value in seconds.
            		var seconds = value;
            		var minutes = 0;
            		var hours = 0;
            		var days = 0;
            		if (seconds >= 60) {
            			minutes = Math.floor(seconds / 60);
            			seconds = 0;
            		}

            		if (minutes >= 60) {
            			hours = Math.floor(minutes / 60);
            			minutes = minutes % 60;
            		}

            		if (hours >= 24) {
            			days = Math.floor(hours / 60);
            			hours = hours % 24;
            		}

            		final_value = '';
            		if (days) {
            			final_value += days + 'd ';
            		}
            		if (hours) {
            			final_value += hours + 'h ';
            		}
            		if (minutes) {
            			final_value += minutes + 'min'
            		}
            		if (seconds) {
            			final_value += seconds + 's'
            		};
            	}

                general.push({'name': key, 'value': final_value});
            }
        } else {
        	if (stat[0] === 'achievement') {
        		var name = key;

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

        		achievements.push({'name': name, 'value': final_value});
        	};
        }
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
        key = dict['name'];
        value = dict['value'];

        name = key.substring('stat.'.length, key.length);
        var row = '<tr id="general-row-' + name + '" class="general-row"><td class="name">' + name + '</td><td class="info">' + value + '</td></tr>'
        loading_stat_general.before(row);
    });

    $.each(mobs, function(index, dict) {
        mobname = dict['name'];

        row = '<tr id="mob-row-' + mobname + '" class="mob-row"><td class="name"></td><td class="killed">0</td><td class="killed-by">0</td></tr>';
        loading_stat_mobs.before(row);
        row = $('#mob-row-' + mobname);
        row.children('.name').text(mobname);

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

        var row = '<tr id="item-row-' + id + '" class="item-row"><td class="name"></td><td class="depleted">0</td><td class="crafted">0</td><td class="used">0</td></tr>';
        loading_stat_item.before(row);
        row = $('#item-row-' + id);
        row.children('.name').text(name);

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

        var row = '<tr id="block-row-' + id + '" class="block-row"><td class="name"></td><td class="crafted">0</td><td class="used">0</td><td class="mined">0</td></tr>';
        loading_stat_block.before(row);
        row = $('#block-row-' + id);
        row.children('.name').text(name);

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

        row = '<tr id="achievement-row-' + name + '" class="achievement-row"><td class="name">' + name + '</td><td class="value">' + dict['value'] + '</td></tr>';
        loading_stat_achievements.before(row);
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
