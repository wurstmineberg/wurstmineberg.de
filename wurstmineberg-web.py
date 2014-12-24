#!/usr/bin/env python3
"""
Wurstmineberg website
"""

import bottle
import os.path

application = bottle.Bottle() # aliased as application for uwsgi to find

working_directory = os.path.dirname(__file__)
bottle.TEMPLATE_PATH = [
    os.path.join(working_directory, 'views/includes'),
    os.path.join(working_directory, 'views')
]

include_files = ['footer', 'header', 'navigation', 'singleservingfooter']
template_variables = {}

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

class StripPathMiddleware:
    """Get that slash out of the request"""
    def __init__(self, a):
        self.app = a
    
    def __call__(self, e, h):
        e['PATH_INFO'] = e['PATH_INFO'].rstrip('/')
        return self.app(e, h)

if __name__ == '__main__':
    bottle.run(app=StripPathMiddleware(application), host='0.0.0.0', port=8081)
