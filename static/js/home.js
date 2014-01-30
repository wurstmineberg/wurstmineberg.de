function displayServerStatus() {
    $.when(API.serverStatus()).done(function(data) {
        if (data.on) {
            get_version_url(data.version, function(version_url) {
                $('#serverinfo').html('The server is currently <strong>online</strong> and running on version <a href="' + version_url + '" style="font-weight: bold;">' + data.version + '</a>, and <span id="peopleCount">(loading) of the (loading) whitelisted players are</span> currently active.<br /><span id="peopleList"></span>');
                getOnlineData(data.list);
            });
        } else {
            $('#serverinfo').html('The server is <strong>offline</strong> right now. For more information, consult the <a href="http://twitter.com/wurstmineberg">Twitter account</a>.');
        }
    }).fail(function(data) {
        $('serverinfo').html('An error occurred while checking the server status. For more information, consult the <a href="https://twitter.com/wurstmineberg">Twitter account</a>.');
    });
}

displayServerStatus();
