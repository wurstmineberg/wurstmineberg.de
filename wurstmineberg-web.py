#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from flask import Flask, render_template, g
from util import templated
app = application = Flask(__name__, template_folder='views')

try:
    import uwsgi
    uwsgi_options = uwsgi.opt
except ImportError:
    uwsgi_options = {}


@app.before_request
def load_template_vars():
	g.host = 'dev.wurstmineberg.de' if uwsgi_options.get('is_dev', False) else 'wurstmineberg.de'


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
    return person

if __name__ == '__main__':
    app.run(debug=False)

