#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from flask import Flask, g, render_template, send_from_directory
from flask.ext.login import LoginManager
from util import templated

import config
from routes import page

import sys
sys.path.append('/opt/py')
from people import PeopleDB

global app
app = application = Flask(__name__)
app.register_blueprint(page)
#login_manager = LoginManager()
#login_manager.init_app(app)

# uwsgi starts the application differently
production = __name__ != '__main__'

try:
    import uwsgi
    uwsgi_options = uwsgi.opt
    has_uwsgi = True
except ImportError:
    uwsgi_options = {}
    has_uwsgi = False

if not production:
    @app.route('/assets/<path:path>')
    def serve_static(path):
        return send_from_directory('assets', path)

    @app.route('/assetserver/<path:path>')
    def serve_assetserver(path):
        return send_from_directory('assetserver', path)


@app.before_request
def before_request():
    is_dev = uwsgi_options.get('is_dev', False)

    # Template variables
    g.host = 'dev.wurstmineberg.de' if is_dev else 'wurstmineberg.de'

    if production:
        import logging
        from logging import FileHandler
        app.config['PROPAGATE_EXCEPTIONS'] = True
        file_handler = FileHandler('/var/log/uwsgi/python/wurstmineberg' + ('-dev' if is_dev else '') + '.log')
        file_handler.setLevel(logging.WARNING)
        app.logger.addHandler(file_handler)

        g.assetserver = 'http://assets.' + g.host
    else:
        g.assetserver = '/assetserver'

    # Initialize database connection
    dbconfig = config.get_db_config()
    g.people = PeopleDB(dbconfig['connectionstring'])

if __name__ == '__main__':
    app.run(debug=True)
