<!DOCTYPE html>
<html>
    <head>
        <title>the people of Wurstmineberg</title>
        {{ !header }}
    </head>
    <body>
        {{ !navigation }}
        <div class="container">
            <div class="panel panel-default">
                <div class="panel-heading">
                    <h3 class="panel-title">All the people</h3>
                </div>
                <div class="panel-body">
                    <p class="lead">Here's a list of all the people who are or have been on the whitelist.</p>
                    <p>Players are ranked chronologically by the date they were invited or whitelisted.</p>
                    <p>To keep player info updated, we kind of rely on the players themselves, so this info may be incomplete or nonsensical. You can use <code>!<a href="http://wiki.wurstmineberg.de/Commands#People">People</a></code> to update some of your info.</p>
                </div>
            </div>
            <div>
                <h2 id="founding">Founding members</h2>
                <table class="table table-responsive people-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Name</th>
                            <th>Info</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-founding-table" class="loading">
                            <td colspan="3">Is this thing on?</td>
                        </tr>
                    </tbody>
                </table>
                <h2 id="later">Later members (pre-<a href="http://wiki.wurstmineberg.de/Server_invitations#History">freeze</a>)</h2>
                <table class="table table-responsive people-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Name</th>
                            <th>Info</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-later-table" class="loading">
                            <td colspan="3">This isn't working…</td>
                        </tr>
                    </tbody>
                </table>
                <h2 id="postfreeze">Later members (post-<a href="http://wiki.wurstmineberg.de/Server_invitations#History">freeze</a>)</h2>
                <table class="table table-responsive people-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Name</th>
                            <th>Info</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-postfreeze-table" class="loading">
                            <td colspan="3">Seriously, something is wrong.</td>
                        </tr>
                    </tbody>
                </table>
                <h2 id="former">Former members</h2>
                <table class="table table-responsive people-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Name</th>
                            <th>Info</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-former-table" class="loading">
                            <td colspan="3">Do you even JavaScript?</td>
                        </tr>
                    </tbody>
                </table>
                <h2 id="guest">Invited people and guests</h2>
                <table class="table table-responsive people-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Name</th>
                            <th>Info</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-guest-table" class="loading">
                            <td colspan="3">(╯°□°)╯︵┻━┻</td>
                        </tr>
                    </tbody>
                </table>
            </div> <!-- table -->
        </div> <!-- /container -->
        {{ !footer }}
        <script src="http://assets.{{host}}/js/people.js"></script>
    </body>
</html>
