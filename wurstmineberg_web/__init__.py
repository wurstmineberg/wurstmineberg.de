#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from flask import Flask, g, render_template, send_from_directory
from flask_bootstrap import Bootstrap
from flask_wtf import CsrfProtect

app = None

def create_app(production):
    global app
    app = Flask(__name__, template_folder='templates/')
    app.config.from_object('wurstmineberg_web.settings')

    from social.apps.flask_app.routes import social_auth
    import wurstmineberg_web.database
    import wurstmineberg_web.views
    import wurstmineberg_web.config
    import wurstmineberg_web.auth
    import wurstmineberg_web.error

    app.register_blueprint(social_auth)
    Bootstrap(app)
    CsrfProtect(app)

    database.init_db()

    if not production:
        import os
        # Because of bugs https://gist.github.com/uniphil/7777590 we need to use absolute paths
        @app.route('/assets/<path:path>')
        def serve_static(path):
            return send_from_directory(os.path.join(app.root_path, 'assets'), path)

        @app.route('/assetserver/<path:path>')
        def serve_assetserver(path):
            return send_from_directory(os.path.join(app.root_path, 'assetserver'), path)

    @app.before_request
    def before_request():
        g.is_dev = uwsgi_options.get('is_dev', False)

        # Template variables
        g.host = 'dev.wurstmineberg.de' if g.is_dev else 'wurstmineberg.de'

        if production:
            g.assetserver = 'http://assets.' + g.host
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

