#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from flask import Flask, g, render_template, send_from_directory
import flask_bootstrap
import flask_wtf

import wurstmineberg_web.util

app = None

def create_app(production):
    global app
    app = Flask(__name__, template_folder='templates/')

    app.url_map.strict_slashes = False
    # load config
    if wurstmineberg_web.util.CONFIG_PATH.exists():
        app.config.update(wurstmineberg_web.util.load_json(wurstmineberg_web.util.CONFIG_PATH))

    import wurstmineberg_web.database
    import wurstmineberg_web.views
    import wurstmineberg_web.auth
    import wurstmineberg_web.error

    flask_bootstrap.Bootstrap(app)
    flask_wtf.CSRFProtect(app)

    database.init_db()

    if not production:
        import os
        # Because of bugs https://gist.github.com/uniphil/7777590 we need to use absolute paths
        @app.route('/assetserver/<path:path>')
        def serve_assetserver(path):
            return send_from_directory(os.path.join(app.root_path, 'assetserver'), path)

    @app.before_request
    def before_request():
        g.is_dev = uwsgi_options.get('is_dev', False)

        # Template variables
        g.host = 'dev.wurstmineberg.de' if g.is_dev else 'wurstmineberg.de'

        if production:
            g.assetserver = 'https://assets.' + g.host
        else:
            g.assetserver = '/assetserver'

    return app


try:
    import uwsgi
    uwsgi_options = uwsgi.opt
    has_uwsgi = True
except ImportError:
    uwsgi_options = {}
    has_uwsgi = False
