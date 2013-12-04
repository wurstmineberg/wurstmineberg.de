function display_leaderboard_stat_data(stat_data, string_data, people) {
    var stats = [];
    var loading_leaderboards = $('#loading-stat-leaderboard-table');

    $.each(stat_data, function(minecraftname, playerstats) {
        player = people.personByMinecraft(minecraftname);

        $.each(playerstats, function(key, value) {
            stat = key.split('.');
            var override = false;
            var add_name = false;
            var found = false;
            var matched_index;
            var stat_to_override;

            var name = stat[1];
            if ('stats' in string_data) {
                if ('general' in string_data['stats']) {
                    if (stat[1] in string_data['stats']['general']) {
                        name = string_data['stats']['general'][stat[1]];
                    };
                };
            }
            
            $.each(stats, function(index, playerstat) {
                if (playerstat['id'] === key) {
                    found = true;
                    if (value > playerstat['value']) {
                        stats[index]['players'] = [player];
                        stats[index]['value'] = value;
                    } else if (value == playerstat['value']) {
                        stats[index]['players'].push(player);
                    }
                    if (value < playerstat['minvalue']) {
                        stats[index]['minplayers'] = [player];
                        stats[index]['minvalue'] = value;
                    } else if (value == playerstat['minvalue']) {
                        stats[index]['minplayers'].push(player);
                    }
                    if (found) {
                        return;
                    }
                }
            });
            
            if (!found) {
                stats.push({
                    'id': key,
                    'name': name,
                    'players': [player],
                    'value': value,
                    'minplayers': [player],
                    'minvalue': value
                });
            };
        });
    });
    
    stats.sort(function(a, b) {
        nameA = a['name'];
        nameB = b['name'];
        return nameA.localeCompare(nameB);
    });
    
    $.each(stats, function(index, data) {
        var key = data['id']
        var stat = key.split('.');
        var name = data['name'];

        var players = data['players'];
        var playerhtml = html_player_list(players);
        var minplayers = data['minplayers'];
        var minplayerhtml = html_player_list(minplayers);
        var value = prettify_stats_value(stat[1], data['value']);
        var minvalue = prettify_stats_value(stat[1], data['minvalue']);

        row = '<tr class="leaderboard-row"><td class="stat">' + name + '</td><td class="leading-player">' + playerhtml + '</td><td class="value">' + value + '</td><td class="trailing-player">' + minplayerhtml + '</td><td class="minvalue">' + minvalue + '</td></tr>';
        loading_leaderboards.before(row);
    });

    $('.loading-stat').remove();
}

function prepare_achievements(achievement_data, item_data) {
    var missing_main_track = {};
    $.each(achievement_data, function(achievement_id, achievement_info) {
        if (achievement_info['track'] == 'main') {
            missing_main_track[achievement_id] = achievement_info;
        }
    });
    while (Object.keys(missing_main_track).length) {
        $.each(missing_main_track, function(achievement_id, achievement_info) {
            var fancy = false;
            if ('fancy' in achievement_info) {
                fancy = achievement_info['fancy'];
            }
            var achievement_image = '/assets/img/grid/' + item_data[achievement_info['icon'].toString()]['image'];
            var achievement_html = '<tr id="achievement-row-' + achievement_id + '"><td><img class="achievement-image' + (fancy ? ' fancy' : '') + '" src="' + achievement_image + '" /></td><td>' + achievement_info['displayname'] + '</td><td>&nbsp;</td>';
            if (achievement_info['requires'] == null) {
                $('#achievement-row-none').after(achievement_html);
                delete missing_main_track[achievement_id];
            } else if ($('#achievement-row-' + achievement_info['requires']).length) {
                $('#achievement-row-' + achievement_info['requires']).after(achievement_html);
                delete missing_main_track[achievement_id];
            }
        });
    }
    $('#achievement-row-loading').remove();
}

function display_achievements_stat_data(achievement_data) {
    //TODO
}

function display_biomes_stat_data(biome_data) {
    //TODO
}

function load_leaderboard_stat_data() {
    $.when(API.statData(), API.stringData(), API.people())
        .done(function(stat_data, string_data, people) {
            display_leaderboard_stat_data(stat_data, string_data, people)
        })
        .fail(function() {
            $('#loading-stat-leaderboard-table').html('<td colspan="7">Error: Could not load api.wurstmineberg.de/server/playerstats/general.json</td>');
        });
}

function load_achievements_stat_data() {
    $.when(API.biomes(), API.itemData()).done(function(biome_data, item_data) {
        $.when(API.achievementData()).done(function(achievement_data) {
            prepare_achievements(achievement_data, item_data);
            display_achievements_stat_data(achievement_data);
        }).fail(function() {
            $('#achievement-row-loading').html('<td colspan="3">Error: Could not load achievements</td>');
        });
        display_biomes_stat_data(achievement_data);
    }).fail(function() {
        $('#loading-achievements-table-biome-track').html('<td colspan="3">Error: Could not load biomes</td>');
    });
}

select_tab_with_id("tab-stats-leaderboard");
bind_tab_events();
load_leaderboard_stat_data();
load_achievements_stat_data();
