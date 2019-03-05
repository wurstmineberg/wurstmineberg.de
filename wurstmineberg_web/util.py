from flask import request, render_template
import functools
import pathlib
import simplejson

BASE_PATH = pathlib.Path('/opt/wurstmineberg')
CONFIG_PATH = BASE_PATH / 'config.json'

def load_json(path):
    with path.open() as f:
        return simplejson.load(f, use_decimal=True)

def templated(template=None):
    def decorator(f):
        @functools.wraps(f)
        def decorated_function(*args, **kwargs):
            template_name = template
            if template_name is None:
                template_name = request.endpoint \
                    .replace('.', '/') + '.html'
            ctx = f(*args, **kwargs)
            if ctx is None:
                ctx = {}
            elif not isinstance(ctx, dict):
                return ctx
            return render_template(template_name, **ctx)
        return decorated_function
    return decorator
