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
                        stats[index] = {'id': key, 'name': name, 'players': [player], 'value': value};
                        return;
                    } else if (value == playerstat['value']) {
                        stats[index]['players'].push(player);
                        return;
                    }
                }
            });

            if (!found) {
                stats.push({'id': key, 'name': name, 'players': [player], 'value': value});
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
        var value = prettify_stats_value(stat[1], data['value']);

        row = '<tr class="leaderboard-row"><td class="stat">' + name + '</td><td class="leading-player">' + playerhtml + '</td><td class="value">' + value + '</td></tr>';
        loading_leaderboards.before(row);
    });

    $('.loading-stat').remove();
}

function load_leaderboard_stat_data() {
	$.when(API.statData(), API.stringData(), API.people())
		.done(function(stat_data, string_data, people) {
			display_leaderboard_stat_data(stat_data, string_data, people)
		})
		.fail(function() {
            $('.loading-stat').html('<td colspan="7">Error: Could not load api.wurstmineberg.de/server/playerstats/general.json</td>');
		});
}

load_leaderboard_stat_data();
bind_tab_events();
select_tab_with_id("tab-stats-leaderboard");
