#!/usr/bin/env python3
"""
Wurstmineberg website
"""

import bottle
import contextlib
import os.path

try:
    import uwsgi
    uwsgi_options = uwsgi.opt
except ImportError:
    uwsgi_options = {}

if uwsgi_options.get('is_dev', False):
    bottle.debug()

class StripPathMiddleware(bottle.Bottle):
    """Get that slash out of the request"""
    def __call__(self, e, *args, **kwargs):
        e['PATH_INFO'] = e['PATH_INFO'].rstrip('/')
        return super().__call__(e, *args, **kwargs)

application = StripPathMiddleware()

working_directory = os.path.dirname(__file__)
bottle.TEMPLATE_PATH = [
    os.path.join(working_directory, 'views/includes'),
    os.path.join(working_directory, 'views')
]

include_files = ['footer', 'header', 'navigation']
template_variables = {
    'host': 'dev.wurstmineberg.de' if uwsgi_options.get('is_dev', False) else 'wurstmineberg.de'
}

for name in include_files:
    with open(os.path.join(working_directory, 'views/includes', name + '.html')) as file:
        template_variables[name] = file.read()

@application.route('/')
@bottle.view('index')
def index():
    return template_variables

@application.route('/about')
@bottle.view('about')
def index():
    return template_variables

@application.route('/stats')
@bottle.view('stats')
def index():
    return template_variables

@application.route('/people')
@bottle.view('people')
def index():
    return template_variables

@application.route('/people/<person:re:[a-z][0-9a-z]{1,15}>')
@bottle.view('people_detail')
def index(person):
    return template_variables

if __name__ == '__main__':
    bottle.run(app=application, host='0.0.0.0', port=8081)
