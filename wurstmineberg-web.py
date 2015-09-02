#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from flask import Flask, render_template, send_from_directory, g
from util import templated

import config

import sys
sys.path.append('/opt/py')
from people import PeopleDB

app = application = Flask(__name__, template_folder='views')

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
    # Template variables
    g.host = 'dev.wurstmineberg.de' if uwsgi_options.get('is_dev', False) else 'wurstmineberg.de'

    if not production:
        g.assetserver = '/assetserver'
    else:
        g.assetserver = 'http://assets.' + g.host

    # Initialize database connection
    dbconfig = config.get_db_config()
    g.people = PeopleDB(dbconfig['connectionstring'])

@app.route('/')
@templated('index.html')
def index():
    return None

@app.route('/about')
@templated()
def about():
    return None

@app.route('/stats')
@templated()
def stats():
    return None

@app.route('/people')
@templated()
def people():
    return None

@app.route('/people/<person>')
@templated('people_detail.html')
def people_detail(person):
    return {'person': person}


if __name__ == '__main__':
    app.run(debug=True)
