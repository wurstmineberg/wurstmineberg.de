function display_leaderboard_stat_data(data) {
    $.when(fetch_string_data()).done(function(string_data) {
        var stats = {};
        var loading_leaderboards = $('#loading-stat-leaderboard-table');

        $.each(data, function(playername, playerstats) {
            $.each(playerstats, function(stat, value) {
                var override = false;
                var add = false;

                if (stat in stats) {
                    if (value > stats[stat]['value']) {
                        override = true;
                    }

                    else if (value == stats[stat]['value']) {
                        add = true;
                    }
                } else {
                    override = true;
                }

                if (override) {
                    stats[stat] = {'players': [playername], 'value': value};
                };

                if (add) {
                    stats[stat]['players'].push(playername)
                };
            });
        });
        
        $.each(stats, function(key, data) {
            stat = key.split('.');
            var name = stat[1];
            if ('stats' in string_data) {
                if ('general' in string_data['stats']) {
                    if (stat[1] in string_data['stats']['general']) {
                        name = string_data['stats']['general'][stat[1]];
                    };
                };
            };

            var players = data['players'];
            var playerhtml = '';
            $.each(players, function(index, name) {
                if (index >= 1) {
                    name = ', ' + name;
                };
                playerhtml += '<span class="player">' + name + '</span>';
            });

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
