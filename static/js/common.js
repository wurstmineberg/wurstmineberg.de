function url_domain(data) {
    var a = document.createElement('a');
    a.href = data;
    return a.hostname;
}

function reddit_user_link(username) {
	return 'https://reddit.com/u/' + username;
}

function twitter_user_link(username) {
	return 'https://twitter.com/' + username;
}

function wiki_user_link(username) {
	username = username.replace(/ /g, '_');
    return 'http://wiki.wurstmineberg.de/User:' + username;
}

// Some string functions to ease the parsing of substrings
String.prototype.startsWith = function(needle)
{
    return(this.indexOf(needle) == 0);
};

String.prototype.endsWith = function(suffix) {
    return this.indexOf(suffix, this.length - suffix.length) !== -1;
};

function linkify_headers() {
	// Do the stuff to the headers to linkify them

	$.each($('h2'), function() {
		$(this).addClass("anchor");
		$(this).append('&nbsp;<a class="tag" href="#' + $(this).attr('id') + '">¶</a>');
	});
	$('h2').hover(function() {
		$(this).children('.tag').css('display', 'inline');
	}, function() {
		$(this).children('.tag').css('display', 'none');
	});
}

function configure_navigation() {
	var navigation_items = $("#navbar-list > li");
	var windowpath = window.location.pathname;

	// Iterate over the list items and change the container of the active nav item to active
	$.each(navigation_items, function() {
		var elementlink = $(this).children($("a"))[0];
		var elementpath = elementlink.getAttribute("href");
		if (elementpath === windowpath) {
			$(this).addClass("active");
		}
	});
}

function set_anchor_height() {
	var navigation_height = $(".navbar").css("height");
	var anchor = $(".anchor");

	anchor.css("padding-top", "+=" + navigation_height);
	anchor.css("margin-top", "-=" + navigation_height);
}

function getServerStatus(on,version) {
    if (on) {
        var versionString = version == null ? "(error)" : ('<a href="http://minecraft.gamepedia.com/Version_history' + ((version.indexOf('pre') != 1 || version.substring(2,3) == 'w') ? '/Development_versions#' : '#') + version + '" style="font-weight: bold;">' + version + '</a>');
        document.getElementById('serverinfo').innerHTML = 'The server is currently <strong>online</strong> and running on version ' + versionString + ', and <span id="peopleCount">(loading) of the (loading) whitelisted players are</span> currently active.<br /><span id="peopleList"></span>';
    } else {
        document.getElementById('serverinfo').innerHTML = "The server is <strong>offline</strong> right now. For more information, consult the <a href='http://twitter.com/wurstmineberg'>Twitter account</a>.";
    }
}

function getOnlineData(list) {
    if (list.length == 1) {
        document.getElementById('peopleCount').innerHTML = 'one of the <span id="whitelistCount">(loading)</span> whitelisted players is';
    } else if (list.length == 0) {
        document.getElementById('peopleCount').innerHTML = 'none of the <span id="whitelistCount">(loading)</span> whitelisted players are';
    } else {
        document.getElementById('peopleCount').innerHTML = list.length + ' of the <span id="whitelistCount">(loading)</span> whitelisted players are';
    }
    $.ajax('assets/serverstatus/people.json', {
        dataType: 'json',
        error: function(request, status, error) {
            document.getElementById('whitelistCount').innerHTML = '(error)';
        },
        success: function(data) {
            document.getElementById('whitelistCount').innerHTML = data.filter(function(person) {
                return (('status' in person ? person['status'] : 'later') != 'former');
            }).length;
        }
    });
    document.getElementById('peopleList').innerHTML = list.map(function(Name) {
        return '<img class="avatar" src="/assets/img/ava/' + Name + '.png" />' + Name;
    }).join(', ');
}

function display_funding_data() {
    $.ajax('/assets/serverstatus/moneys.json', {
        dataType: 'json',
        error: function(request, status, error) {
            $('.funding-month').html('(error)');
            $('.funding-progressbar').removeClass('active');
            $('.funding-progressbar').children('.progress-bar').addClass('progress-bar-danger');
        },
        success: function(data) {
            $('.funding-progressbar').removeClass('active progress-striped');
            $('.funding-progressbar').empty();
            var funding_total = 0.0;
            data['history'].forEach(function(transaction) {
            	funding_total += transaction['amount'];
            });

            var today = new Date();
            var year = today.getFullYear();
            var month = today.getMonth();
            var spending_monthly = Math.abs(data['spending_monthly']);

            while (funding_total >= spending_monthly) {
                month++;
                if (month >= 12) {
                    year++;
                    month = 0;
                }

                funding_total -= spending_monthly;
            }

            var months = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];
            var following_year = month == 11 ? year + 1 : year;
            $('.funding-month').html(months[month] + ' to ' + months[(month + 1) % 12] + ' ' + following_year);

            var percent = 0;
            if (funding_total > 0.0) {
                percent = Math.floor(funding_total * 100 / spending_monthly);
                $('.funding-progressbar').append('<div class="progress-bar progress-bar-success" style="width: ' + percent + '%;"><span class="sr-only">' + percent + '% funded</span></div>');
            }

            var expected_total = data['funding_monthly'];

            if (percent < 100) {
                data['history'].forEach(function(transaction) {
                    if (transaction['type'] == 'player-monthly') {
                        var transaction_year = transaction['date'].split('-')[0];
                        var transaction_month = transaction['date'].split('-')[1];
                        var transaction_day = transaction['date'].split('-')[2];
                        if (transaction_day < data['billing_dom']) {
                            if ((transaction_month - 1 == month && transaction_year == year) || (month == 12 && transaction_month == 1 && transaction_year - 1 == year)) {
                                expected_total -= transaction['amount'];
                            }
                        } else if (transaction_year == year && transaction_month == month) {
                            expected_total -= transaction['amount'];
                        }
                    }
                });

                var expected_percent = Math.max(0, Math.min(100 - percent, Math.floor(expected_total * 100 / data['spending_monthly'])));
                $('.funding-progressbar').append('<div class="progress-bar progress-bar-warning" style="width: ' + expected_percent + '%;"><span class="sr-only">' + expected_percent + '% expected</span></div>');
            }
        }
    });
}

// Run by default
linkify_headers();
configure_navigation();
set_anchor_height();
display_funding_data();
