import flask
import flask_login
import functools
import requests

import wurstmineberg_web
import wurstmineberg_web.auth
import wurstmineberg_web.models
import wurstmineberg_web.util
import wurstmineberg_web.views

def key_or_member_optional(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        if flask_login.current_user.is_active:
            flask.g.user = flask_login.current_user
        elif wurstmineberg_web.models.Person.from_api_key() is not None:
            flask.g.user = wurstmineberg_web.models.Person.from_api_key()
        else:
            flask.g.user = wurstmineberg_web.auth.AnonymousUser()
        return f(*args, **kwargs)

    return wrapper

def key_or_member_required(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        if flask_login.current_user.is_active:
            flask.g.user = flask_login.current_user
        elif wurstmineberg_web.models.Person.from_api_key() is not None:
            flask.g.user = wurstmineberg_web.models.Person.from_api_key()
        else:
            flask.g.user = wurstmineberg_web.auth.AnonymousUser()
        if flask.g.user is not None and flask.g.user.is_active:
            return f(*args, **kwargs)
        return flask.Response(
            "You don't have permission to access this endpoint, either because you're not a server member or because you haven't entered your API key.",
            401,
            {'WWW-Authenticate': 'Basic realm="wurstmineberg.de API key required"'}
        ) #TODO template

    return wrapper

@wurstmineberg_web.views.index.child('api', 'API')
@key_or_member_optional
def api_index():
    return flask.redirect((flask.g.view_node / 'v3').url)

@api_index.child('v3', 'version 3')
@wurstmineberg_web.util.templated('api-index.html')
def api_v3_index():
    pass

@api_v3_index.child('discord')
def api_discord_index():
    pass

@api_discord_index.child('voice-state.json', decorators=[key_or_member_required])
def discord_voice_state():
    with (wurstmineberg_web.util.BASE_PATH / 'discord' / 'voice-state.json').open() as f:
        return flask.Response(f.read(), mimetype='application/json')

@api_v3_index.child('money')
def api_money_index():
    pass

@api_money_index.child('overview.json')
def api_money_overview():
    response = requests.get('https://nightd.fenhl.net/wurstmineberg/money/overview.json', auth=('wurstmineberg', wurstmineberg_web.app.config['night']['password']))
    response.raise_for_status()
    return flask.Response(response.text, mimetype='application/json')

@api_money_index.child('transactions.json')
def api_money_transactions():
    response = requests.get('https://nightd.fenhl.net/wurstmineberg/money/transactions.json', auth=('wurstmineberg', wurstmineberg_web.app.config['night']['password']))
    response.raise_for_status()
    return flask.Response(response.text, mimetype='application/json')
