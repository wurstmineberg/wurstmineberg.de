import copy
import flask
import flask_login
import flask_view_tree
import functools
import mcanvil
import nbt.nbt
import pathlib
import playerhead
import requests
import simplejson
import tempfile
import time
import wrapt

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
            tmp = tempfile.NamedTemporaryFile(suffix='png')
            f(*args, **kwargs).save(tmp, format='PNG')
            tmp.seek(0)
            return flask.send_file(tmp, mimetype='image/png')

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

def json_children(node, var_converter=flask_view_tree.identity, *args, **kwargs):
    class JsonStem(wrapt.ObjectProxy):
        @property
        def url_part(self):
            return '{}.json'.format(self.__wrapped__.url_part)

    def json_var_converter(x):
        if x.endswith('.json'):
            return JsonStem(var_converter(x[:-len('.json')]))
        else:
            raise ValueError('URL must end with .json')

    def decorator(f):
        @node.children(json_var_converter, *args, **kwargs)
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            result = simplejson.dumps(f(*args, **kwargs), sort_keys=True, indent=4, use_decimal=True)
            return flask.Response(result, mimetype='application/json')

        wrapper.raw = f
        return wrapper

    return decorator

def nbtfile_to_dict(filename, *, add_metadata=True):
    """Generates a JSON-serializable value from a path (string or pathlib.Path) representing a NBT file.
    Keyword-only arguments:
    add_metadata -- If true, converts the result to a dict and adds the .apiTimeLastModified and .apiTimeResultFetched fields.
    """
    if isinstance(filename, pathlib.Path):
        path = filename
        filename = str(filename)
    else:
        path = pathlib.Path(filename)
    nbt_file = nbt.nbt.NBTFile(filename)
    nbt_dict = nbt_to_dict(nbt_file)
    if add_metadata:
        if not isinstance(nbt_dict, dict):
            nbt_dict = {'data': nbt_dict}
        if 'apiTimeLastModified' not in nbt_dict:
            nbt_dict['apiTimeLastModified'] = path.stat().st_mtime
        if 'apiTimeResultFetched' not in nbt_dict:
            nbt_dict['apiTimeResultFetched'] = time.time()
    return nbt_dict

def nbt_to_dict(nbt_file):
    """Generates a JSON-serializable value from an nbt.nbt.NBTFile object."""
    dict = {}
    is_collection = False
    is_dict = False
    collection = []
    for tag in nbt_file.tags:
        if hasattr(tag, 'tags'):
            if tag.name is None or tag.name == '':
                collection.append(nbt_to_dict(tag))
                is_collection = True
            else:
                dict[tag.name] = nbt_to_dict(tag)
                is_dict = True
        else:
            value = tag.value
            if isinstance(value, bytearray):
                value = list(value)
            if tag.name is None or tag.name == '':
                collection.append(value)
                is_collection = True
            else:
                dict[tag.name] = value
                is_dict = True
    if is_collection and is_dict:
        dict['collection'] = collection
    if is_dict:
        return dict
    else:
        return collection

def nbt_child(node, name, *args, **kwargs):
    def decorator(f):
        @functools.wraps(f)
        def nbt_filed(*args, **kwargs):
            result = f(*args, **kwargs)
            if isinstance(result, pathlib.Path):
                return nbt.nbt.NBTFile(str(result))
            elif isinstance(result, nbt.nbt.NBTFile):
                return result
            else:
                raise NotImplementedError('Cannot convert value of type {} to NBTFile'.format(type(result)))

        @functools.wraps(f)
        def dict_encoded(*args, **kwargs):
            result = f(*args, **kwargs)
            if isinstance(result, pathlib.Path):
                return nbtfile_to_dict(result)
            elif isinstance(result, nbt.nbt.NBTFile):
                return nbt_to_dict(result)
            else:
                raise NotImplementedError('Cannot convert value of type {} to JSON'.format(type(result)))

        @node.child(name + '.json', view_name='{}_json'.format(f.__name__), *args, **kwargs)
        @functools.wraps(f)
        def json_encoded(*args, **kwargs):
            result = simplejson.dumps(dict_encoded(*args, **kwargs), sort_keys=True, indent=4, use_decimal=True)
            return flask.Response(result, mimetype='application/json')

        @node.child(name + '.dat', *args, **kwargs)
        @functools.wraps(f)
        def raw_nbt(*args, **kwargs):
            result = f(*args, **kwargs)
            if isinstance(result, pathlib.Path):
                result = nbt.nbt.NBTFile(str(result)) #TODO static file optimization
            if isinstance(result, nbt.nbt.NBTFile):
                buf = io.BytesIO()
                result.write_file(fileobj=buf)
                return flask.Response(buf, mimetype='application/x-minecraft-nbt')
            else:
                raise NotImplementedError('Cannot convert value of type {} to NBT'.format(type(result)))

        pass #TODO add HTML view endpoint

        nbt_filed.dict = dict_encoded # make Python-dict-encoded NBT available for Python code
        nbt_filed.json = json_encoded # make JSON-encoded NBT available for Python code
        nbt_filed.dat = raw_nbt # make raw NBT available for Python code
        return nbt_filed

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
    for uid, person_data in db['people'].items():
        person = wurstmineberg_web.models.Person.from_snowflake_or_wmbid(uid)
        #TODO copy these patches to people.py
        if 'gravatar' in person_data:
            del person_data['gravatar']
        if person.discorddata is not None:
            person_data['discord'] = copy.deepcopy(person.discorddata)
            person_data['discord']['snowflake'] = person.snowflake
        person_data['name'] = person.display_name
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

@api_world_index.child('dim')
def api_world_dimensions(world):
    pass

@api_world_dimensions.children(wurstmineberg_web.models.Dimension.from_url_part, iterable=wurstmineberg_web.models.Dimension)
def api_world_dimension_index(world, dimension):
    pass

@api_world_dimension_index.child('chunk')
def api_chunks_index(world, dimension):
    pass

@api_chunks_index.children(int)
def api_chunks_x(world, dimension, x):
    pass

@api_chunks_x.children(int)
def api_chunks_y(world, dimension, x, y):
    pass

@json_children(api_chunks_y, int)
def api_chunk(world, dimension, x, y, z):
    def nybble(data, idx):
        result = data[idx // 2]
        if idx % 2 == 0:
            return result & 15
        else:
            return result >> 4

    region = mcanvil.Region(world.region_path(dimension) / 'r.{}.{}.mca'.format(x // 32, z // 32))
    column = region.chunk_column(x, z).data
    for section in column['Level']['Sections']:
        if section['Y'] == y:
            break
    else:
        section = None
    with pathlib.Path('/opt/git/github.com/wurstmineberg/assets.wurstmineberg.de/master/json/biomes.json').open() as biomes_file:
        biomes = simplejson.load(biomes_file, use_decimal=True)
    with pathlib.Path('/opt/git/github.com/wurstmineberg/assets.wurstmineberg.de/master/json/items.json').open() as items_file:
        items = simplejson.load(items_file, use_decimal=True)
    layers = []
    for layer in range(16):
        block_y = y * 16 + layer
        rows = []
        for row in range(16):
            block_z = z * 16 + row
            blocks = []
            for block in range(16):
                block_x = x * 16 + block
                block_info = {
                    'x': block_x,
                    'y': block_y,
                    'z': block_z
                }
                if 'Biomes' in column['Level']:
                    block_info['biome'] = biomes['biomes'][str(column['Level']['Biomes'][16 * row + block])]['id']
                if section is not None:
                    block_index = 256 * layer + 16 * row + block
                    block_id = section['Blocks'][block_index]
                    if 'Add' in section:
                        block_id += nybble(section['Add'], block_index) << 8
                    block_info['id'] = block_id
                    for plugin, plugin_items in items.items():
                        for item_id, item_info in plugin_items.items():
                            if 'blockID' in item_info and item_info['blockID'] == block_id:
                                block_info['id'] = '{}:{}'.format(plugin, item_id)
                                break
                    block_info['damage'] = nybble(section['Data'], block_index)
                    block_info['blockLight'] = nybble(section['BlockLight'], block_index)
                    block_info['skyLight'] = nybble(section['SkyLight'], block_index)
                blocks.append(block_info)
            rows.append(blocks)
        layers.append(rows)
    if 'Entities' in column['Level']:
        for entity in column['Level']['Entities']:
            if y * 16 <= entity['Pos'][1].value < y * 16 + 16: # make sure the entity is in the right section
                block_info = layers[int(entity['Pos'][1].value) & 15][int(entity['Pos'][2].value) & 15][int(entity['Pos'][0].value) & 15]
                if 'entities' not in block_info:
                    block_info['entities'] = []
                block_info['entities'].append(nbt_to_dict(entity))
    if 'TileEntities' in column['Level']:
        for tile_entity in column['Level']['TileEntities']:
            if y * 16 <= tile_entity['y'].value < y * 16 + 16: # make sure the entity is in the right section
                block_info = layers[tile_entity['y'].value & 15][tile_entity['z'].value & 15][tile_entity['x'].value & 15]
                del tile_entity['x']
                del tile_entity['y']
                del tile_entity['z']
                if 'tileEntities' in block_info:
                    block_info['tileEntities'].append(nbt_to_dict(tile_entity))
                elif 'tileEntity' in block_info:
                    block_info['tileEntities'] = [block_info['tileEntity'], nbt_to_dict(tile_entity)]
                    del block_info['tileEntity']
                else:
                    block_info['tileEntity'] = nbt_to_dict(tile_entity)
    return layers

@api_world_index.child('player')
def api_world_players_index(world):
    pass

@api_world_players_index.children(wurstmineberg_web.models.Person.from_snowflake_or_wmbid)
def api_world_player(world, player):
    pass

@nbt_child(api_world_player, 'playerdata')
def api_player_data(world, player):
    return world.world_path / 'playerdata' / '{}.dat'.format(player.minecraft_uuid)

@json_child(api_world_index, 'status')
def api_world_status(world):
    return {
        'main': world.is_main,
        'running': world.is_running,
        'version': world.version,
        'list': [person.snowflake_or_wmbid for person in world.online_players]
    }
