function display_leaderboard_stat_data(data) {
    $.when(fetch_string_data()).done(function(string_data) {
        var stats = {};
        var loading_leaderboards = $('#loading-stat-leaderboard-table');

        $.each(data, function(playername, playerstats) {
            $.each(playerstats, function(stat, value) {
                var override = false;
                if (stat in stats) {
                    if (value > stats[stat]) {
                        override = true;
                    }
                } else {
                    override = true;
                }

                stats[stat] = {'player': playername, 'value': value};
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

            var player = data['player'];
            var value = prettify_stats_value(stat[1], data['value']);

            row = '<tr class="leaderboard-row"><td class="stat">' + name + '</td><td class="leading-player">' + player + '</td><td class="value">' + value + '</td></tr>';
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
