function display_leaderboard_stat_data(data) {
    $.when(fetch_string_data(), fetch_people_data()).done(function(string_data, player_data) {
        string_data = string_data[0]
        player_data = player_data[0]

        var stats = [];
        var loading_leaderboards = $('#loading-stat-leaderboard-table');

        $.each(data, function(playername, playerstats) {
            playername = minecraft_nick_to_username(playername, player_data)
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
                            stats[index] = {'id': key, 'name': name, 'players': [playername], 'value': value};
                            return;
                        } else if (value == playerstat['value']) {
                            stats[index]['players'].push(playername);
                            return;
                        }
                    }
                });

                if (!found) {
                    stats.push({'id': key, 'name': name, 'players': [playername], 'value': value});
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
            var playerhtml = html_player_list(players, player_data);
            var value = prettify_stats_value(stat[1], data['value']);

            row = '<tr class="leaderboard-row"><td class="stat">' + name + '</td><td class="leading-player">' + playerhtml + '</td><td class="value">' + value + '</td></tr>';
            loading_leaderboards.before(row);
        });

        $('.loading-stat').remove();
    });
}

function load_leaderboard_stat_data(minecraft) {
    $.ajax('//api.wurstmineberg.de/server/playerstats/general.json', {
        dataType: 'json',
        error: function(request, status, error) {
            $('.loading-stat').html('<td colspan="7">Error: Could not load ' + minecraft + '.json</td>');
        },
        success: function(data) {
            display_leaderboard_stat_data(data);
        }
    });
}

load_leaderboard_stat_data();
bind_tab_events();
select_tab_with_id("tab-stats-leaderboard");
