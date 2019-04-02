import flask
import functools
import pathlib
import simplejson

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

def templated(template=None):
    def decorator(f):
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            template_name = template
            if template_name is None:
                template_name = flask.request.endpoint \
                    .replace('.', '/') + '.html'
            ctx = f(*args, **kwargs)
            if ctx is None:
                ctx = {}
            elif not isinstance(ctx, dict):
                return ctx
            return flask.render_template(template_name, **ctx)

        return wrapper

    return decorator
