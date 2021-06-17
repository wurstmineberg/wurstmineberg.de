import functools
import pathlib

import flask # PyPI: Flask
import jinja2 # PyPI: Jinja2
import simplejson # PyPI: simplejson

BASE_PATH = pathlib.Path('/opt/wurstmineberg')
CONFIG_PATH = BASE_PATH / 'config.json'

def load_json(path):
    with path.open() as f:
        return simplejson.load(f, use_decimal=True)

def redirect_empty(url_f):
    def decorator(f):
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            result = f(*args, **kwargs)
            if result is None:
                return flask.redirect(url_f(flask.g.view_node))
            else:
                return result

        return wrapper

    return decorator

def render_template(template_name=None, **kwargs):
    if template_name is None:
        template_path = '{}.html.j2'.format(flask.request.endpoint.replace('.', '/'))
    else:
        template_path = '{}.html.j2'.format(template_name.replace('.', '/'))
    return jinja2.Markup(flask.render_template(template_path, **kwargs))

def template(template_name=None):
    def decorator(f):
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            context = f(*args, **kwargs)
            if context is None:
                context = {}
            elif not isinstance(context, dict):
                return context
            return render_template(template_name, **context)

        return wrapper

    return decorator

def setup(app):
    @app.template_filter()
    def ymd(value):
        return f'{value:%Y-%m-%d}'
