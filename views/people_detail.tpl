<!DOCTYPE html>
<html>
    <head>
        <title>profile page — Wurstmineberg</title>
        {{ !header }}
    </head>
    <body>
        {{ !navigation }}
        <div class="container">
            <div class="panel panel-default">
                <div class="panel-heading">
                    <img id="avatar" class="hidden" src="" alt="avatar" />
                    <h3 id="username" class="panel-title panel-loading">Loading data…</h3>
                </div>
                <div class="panel-body">
                    <div class="lead">
                        <img id="head" class="hidden img-rounded" src="" alt="head" />
                        <div id="user-info">
                            <p id="user-description" class="panel-loading">Loading user data…</p>
                            <p id="social-links" class="hidden"></p>
                            <div class="inventory-opt-out pull-left">
                                <h2 id="inventory">Inventory</h2>
                                <table id="main-inventory" class="inventory-table">
                                    <tbody>
                                        <tr class="loading">
                                            <td>loading…</td>
                                        </tr>
                                    </tbody>
                                </table>
                                <div style="height: 29px;"></div>
                                <table id="hotbar-table" class="inventory-table">
                                    <tbody>
                                        <tr class="loading">
                                            <td>loading…</td>
                                        </tr>
                                    </tbody>
                                </table>
                            </div>
                            <div class="inventory-opt-out">
                                <h2 id="enderchest">Ender chest</h2>
                                <table id="ender-chest-table" class="inventory-table">
                                    <tbody>
                                        <tr class="loading">
                                            <td>loading…</td>
                                        </tr>
                                    </tbody>
                                </table>
                                <div style="height: 29px;"></div>
                                <table id="offhand-slot-table" class="inventory-table" style="float: right;">
                                    <tr class="loading">
                                        <td>loading…</td>
                                    </tr>
                                </table>
                                <table id="armor-table" class="inventory-table">
                                    <tbody>
                                        <tr class="loading">
                                            <td>loading…</td>
                                        </tr>
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <h2>Statistics</h2>
            <ul id="pagination" class="nav nav-tabs">
                <li><a id="tab-stats-profile" class="tab-item" href="#profile">Profile</a></li>
                <li><a id="tab-stats-general" class="tab-item" href="#general">General</a></li>
                <li><a id="tab-stats-blocks" class="tab-item" href="#blocks">Blocks</a></li>
                <li><a id="tab-stats-items" class="tab-item" href="#items">Items</a></li>
                <li><a id="tab-stats-mobs" class="tab-item" href="#mobs">Mobs</a></li>
                <li><a id="tab-stats-achievements" class="tab-item" href="#achievements">Achievements</a></li>
                <li><a id="tab-stats-minigames" class="tab-item" href="#minigames">Minigames</a></li>
            </ul>
            <div id="stats-profile" class="section">
                <table id="stats-profile-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>Info</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr class="profile-stat-row" id="profile-stat-row-dow">
                            <td>Date of Whitelisting</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-fav-color">
                            <td>Favorite Color</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-fav-item">
                            <td>Favorite Item</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-invited-by">
                            <td>Invited By</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-last-death">
                            <td>Last Death</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-last-seen">
                            <td>Last Seen</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-people-invited-prefreeze">
                            <td>People “Invited” (pre-<a href="http://wiki.wurstmineberg.de/Server_invitations#History">freeze</a>)</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-people-invited">
                            <td>People Invited (post-freeze)</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr class="profile-stat-row" id="profile-stat-row-status">
                            <td>Status</td>
                            <td class="value">(loading)</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-general" class="section hidden">
                <table id="stats-general-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>Stat</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-stat-general-table" class="loading-stat">
                            <td colspan="2">Loading stat data…</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-blocks" class="section hidden">
                <table id="stats-blocks-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Block</th>
                            <th>Times Crafted</th>
                            <th>Times Used</th>
                            <th>Times Mined</th>
                            <th>Times Dropped</th>
                            <th>Times Picked Up</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-stat-blocks-table" class="loading-stat">
                            <td colspan="5">Loading stat data…</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-items" class="section hidden">
                <table id="stats-items-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Item</th>
                            <th>Times Crafted</th>
                            <th>Times Used</th>
                            <th>Times Depleted</th>
                            <th>Times Dropped</th>
                            <th>Times Picked Up</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-stat-items-table" class="loading-stat">
                            <td colspan="5">Loading stat data…</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-mobs" class="section hidden">
                <table id="stats-mobs-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>Mob</th>
                            <th>Killed</th>
                            <th>Killed By</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-stat-mobs-table" class="loading-stat">
                            <td colspan="3">Loading stat data…</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-achievements" class="section hidden">
                <table id="stats-achievements-table" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>&nbsp;</th>
                            <th>Achievement</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="loading-stat-achievements-table" class="loading-stat">
                            <td colspan="3">Loading stat data…</td>
                        </tr>
                    </tbody>
                </table>
            </div>
            <div id="stats-minigames" class="section hidden">
                <h2>Achievement Run</h2>
                <table id="minigames-stats-table-achievementrun" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>Stat</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="minigames-stat-row-achievementrun-place">
                            <td>Rank</td>
                            <td class="value">(loading)</td>
                        </tr>
                    </tbody>
                </table>
                <h2>Death Games</h2>
                <table id="minigames-stats-table-deathgames" class="table table-responsive stats-table">
                    <thead>
                        <tr>
                            <th>Stat</th>
                            <th>Value</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr id="minigames-stat-row-deathgames-kills">
                            <td>Kills</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-deaths">
                            <td>Deaths</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-diamonds">
                            <td>Diamonds earned (kills minus deaths)</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-attacks">
                            <td>Attacks total</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-attacks-success">
                            <td>Successful attacks</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-attacks-fail">
                            <td>Failed attacks</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-defense">
                            <td>Defenses total</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-defense-success">
                            <td>Successful defenses</td>
                            <td class="value">(loading)</td>
                        </tr>
                        <tr id="minigames-stat-row-deathgames-defense-fail">
                            <td>Failed defenses</td>
                            <td class="value">(loading)</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
        {{ !footer }}
        <script src="http://assets.{{host}}/js/profile.js"></script>
    </body>
</html>
