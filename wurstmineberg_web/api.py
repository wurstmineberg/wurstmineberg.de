import copy
import flask
import flask_login
import flask_view_tree
import functools
import playerhead
import requests
import simplejson

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

def image_child(node, name, *args, **kwargs): #TODO caching
    def decorator(f):
        @node.child(name + '.png', *args, **kwargs)
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            return flask.send_file(f(*args, **kwargs)) #TODO MIME type

        wrapper.raw = f
        return wrapper

    return decorator

def json_child(node, name, *args, **kwargs):
    def decorator(f):
        @node.child(name + '.json', *args, **kwargs)
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            result = simplejson.dumps(f(*args, **kwargs), sort_keys=True, indent=4, use_decimal=True)
            return flask.Response(result, mimetype='application/json')

        wrapper.raw = f
        return wrapper

    return decorator

@wurstmineberg_web.views.index.child('api', 'API', decorators=[key_or_member_optional])
def api_index():
    return flask.redirect((flask.g.view_node / 'v3').url)

@api_index.child('v3', 'version 3', decorators=[wurstmineberg_web.util.redirect_empty(lambda view_node: flask.url_for('api_index'))])
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

@json_child(api_money_index, 'overview')
def money_overview():
    response = requests.get('https://nightd.fenhl.net/wurstmineberg/money/overview.json', auth=('wurstmineberg', wurstmineberg_web.app.config['night']['password']))
    response.raise_for_status()
    return response.json()

@json_child(api_money_index, 'transactions')
def money_transactions():
    #TODO show which transactions are mine
    response = requests.get('https://nightd.fenhl.net/wurstmineberg/money/transactions.json', auth=('wurstmineberg', wurstmineberg_web.app.config['night']['password']))
    response.raise_for_status()
    return response.json()

@json_child(api_v3_index, 'people')
def api_people():
    db = copy.deepcopy(wurstmineberg_web.models.Person.obj_dump(version=3))
    for uid, person in db['people'].items():
        #TODO copy these patches to people.py
        if 'gravatar' in person:
            del person['gravatar']
        person['name'] = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(uid).display_name
    return db

@api_v3_index.child('person')
def api_people_index():
    pass

@api_people_index.children(wurstmineberg_web.models.Person.from_snowflake_or_wmbid)
def api_person(person):
    pass

@json_child(api_person, 'avatar')
def api_avatars(person):
    return person.avatar

@api_person.child('skin')
def api_player_skins(person):
    pass

@image_child(api_player_skins, 'head')
def api_player_head(person):
    return playerhead.head(person.minecraft_uuid)

@api_v3_index.child('world')
def api_worlds_index():
    pass

@api_worlds_index.children(wurstmineberg_web.models.World)
def api_world_index(world):
    pass

@json_child(api_world_index, 'status')
def api_world_status(world):
    return {
        'main': world.is_main,
        'running': world.is_running,
        'version': world.version
        #TODO list
        #TODO whitelist
    }
