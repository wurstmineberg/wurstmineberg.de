#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Wurstmineberg Website
"""


import bottle
import os

app = application = bottle.Bottle()  # aliased as application for uwsgi to find

workingDirectory = os.path.dirname(__file__)
bottle.TEMPLATE_PATH = [
    os.path.join(workingDirectory, 'views/includes'),
    os.path.join(workingDirectory, 'views')]


includeFiles = ['footer', 'header', 'navigation', 'singleservingfooter']
templateVariables = {}

for name in includeFiles:
    with open(os.path.join(workingDirectory, 'views/includes', name + '.html'), encoding='utf8') as file:
        templateVariables[name] = file.read()


@app.route('/')
@bottle.view('index')
def index():
    return templateVariables


@app.route('/about')
@bottle.view('about')
def index():
    return templateVariables


@app.route('/stats')
@bottle.view('stats')
def index():
    return templateVariables


@app.route('/people')
@bottle.view('people')
def index():
    return templateVariables


@app.route('/people/:person')
@bottle.view('peopleDetail')
def index(person):
    return templateVariables


class StripPathMiddleware:

    """Get that slash out of the request"""

    def __init__(self, a):
        self.app = a

    def __call__(self, e, h):
        e['PATH_INFO'] = e['PATH_INFO'].rstrip('/')
        return self.app(e, h)

if __name__ == '__main__':
    bottle.run(app=StripPathMiddleware(app), host='0.0.0.0', port=8081)
