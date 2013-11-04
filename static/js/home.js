function displayServerStatus() {
	$.when(API.serverStatus())
		.done(function(data) {
			getServerStatus(data.on, data.version);
            getOnlineData(data.list);
		})
		.fail(function(data) {
			document.getElementById('serverinfo').innerHTML = 'An error occurred while checking the server status. For more information, consult the <a href="https://twitter.com/wurstmineberg">Twitter account</a>.';
		})
}

displayServerStatus()
