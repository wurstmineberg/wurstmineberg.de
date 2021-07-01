import copy
import functools
import pathlib
import tempfile
import time

import flask # PyPI: Flask
import flask_login # PyPI: Flask-Login
import nbt.nbt # PyPI: NBT
import requests # PyPI: requests
import simplejson # PyPI: simplejson
import wrapt # PyPI: wrapt

import flask_view_tree # https://github.com/fenhl/flask-view-tree
import mcanvil # https://github.com/wurstmineberg/python-anvil
import playerhead # https://github.com/wurstmineberg/playerhead

import wurstmineberg_web
import wurstmineberg_web.auth
import wurstmineberg_web.models
import wurstmineberg_web.util
import wurstmineberg_web.views

class FileExtError(ValueError):
    def __init__(self):
        super().__init__('URL must end with .json')

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
            result = simplejson.dumps(f(*args, **kwargs), sort_keys=True, indent=4)
            return flask.Response(result, mimetype='application/json')

        wrapper.raw = f
        return wrapper

    return decorator

def json_children(node, var_converter=flask_view_tree.identity, *args, **kwargs):
    class JsonStem(wrapt.ObjectProxy):
        @property
        def url_part(self):
            if hasattr(self.__wrapped__, 'url_part'):
                return '{}.json'.format(self.__wrapped__.url_part)
            else:
                return '{}.json'.format(self.__wrapped__)

    def json_var_converter(x):
        if x.endswith('.json'):
            return JsonStem(var_converter(x[:-len('.json')]))
        else:
            raise FileExtError()

    def decorator(f):
        @node.children(json_var_converter, *args, **kwargs)
        @functools.wraps(f)
        def wrapper(*args, **kwargs):
            result = simplejson.dumps(f(*args, **kwargs), sort_keys=True, indent=4)
            return flask.Response(result, mimetype='application/json')

        @wrapper.catch_init(FileExtError)
        def json_catch_init(exc, value):
            return wurstmineberg_web.util.render_template('api-ext-404'), 404

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
            result = simplejson.dumps(dict_encoded(*args, **kwargs), sort_keys=True, indent=4)
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
@wurstmineberg_web.util.template('api-index')
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
        person_data['name'] = person.name
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

@image_child(api_player_skins, 'front')
def api_player_skin_front(person):
    return playerhead.body(person.minecraft_uuid)

@image_child(api_player_skins, 'head')
def api_player_head(person):
    return playerhead.head(person.minecraft_uuid)

@api_v3_index.child('server')
def api_server_index():
    pass

@json_child(api_server_index, 'worlds')
def api_worlds():
    """Returns an object mapping existing world names to short status summaries (like those returned by /world/<world>/status.json but without the lists of online players)"""
    result = {}
    for world in wurstmineberg_web.models.World:
        result[world.name] = {
            'main': world.is_main,
            'running': world.is_running,
            'version': world.version
        }
        if 'list' in flask.request.args:
            result[world.name]['list'] = [person.snowflake_or_wmbid for person in world.online_players]
    return result

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
    def block_from_states_and_palette(data_version, states, palette, block_index):
        if data_version >= 2529: # starting in 20w17a, indices are no longer split across multiple longs
            bits_per_index = max((len(palette) - 1).bit_length(), 4)
            indexes_per_long = 64 // bits_per_index
            containing_long, index_offset = divmod(block_index, indexes_per_long)
            bit_offset = index_offset * bits_per_index
            mask = 2 ** bits_per_index - 1
            index = (states[containing_long] >> bit_offset) & mask
            return palette[index]
        else:
            bits_per_index = max((len(palette)-1).bit_length(), 4)

            bit_index = block_index * bits_per_index
            containing_index = bit_index // 64
            offset = bit_index % 64

            bit_end_index = bit_index + bits_per_index
            containing_end_index = bit_end_index // 64
            end_offset = bit_end_index % 64

            source_fields = containing_end_index - containing_index
            mask = (2**bits_per_index-1)

            ii = 0
            index = 0
            while ii <= source_fields:
                state_index = containing_index+ii
                try:
                    field = states[state_index]
                except IndexError:
                    field = 0
                field %= 2**64
                part = field << (64*ii)
                index |= part
                ii += 1
            index >>= offset
            index &= mask

            return palette[index]

    region = mcanvil.Region(world.region_path(dimension) / 'r.{}.{}.mca'.format(x // 32, z // 32))
    column = region.chunk_column(x, z).data
    for section in column['Level']['Sections']:
        if section['Y'].value == y:
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
                    if len(column['Level']['Biomes']) == 1024: # starting in 19w36a, biomes are stored per 4x4x4 cube
                        block_info['biome'] = biomes['biomes'][str(column['Level']['Biomes'][16 * (block_y // 4) + 4 * (row // 4) + (block // 4)])]['id']
                    else: # before 19w36a, biomes were stored per block column
                        block_info['biome'] = biomes['biomes'][str(column['Level']['Biomes'][16 * row + block])]['id']
                if section is not None:
                    block_index = 256 * layer + 16 * row + block
                    palette = section.get('Palette')
                    block_states = section.get('BlockStates')
                    if palette and block_states:
                        block = block_from_states_and_palette(column['DataVersion'].value, block_states, palette, block_index)
                        block_id = block["Name"].value
                        block_info['id'] = block_id
                        if 'Add' in section:
                            block_id += nybble(section['Add'], block_index) << 8
                        plugin, name = block_id.split(":", 1)
                        info = items.get(plugin, {}).get(name, None)
                        if not info is None:
                            block_info['info'] = info
                        #for plugin, plugin_items in items.items():
                            #for item_id, item_info in plugin_items.items():
                                #if 'blockID' in item_info and item_info['blockID'] == block_id:
                                    #block_info['info'] = item_info
                                    #break
                    if "Data" in section:
                        block_info['damage'] = nybble(section['Data'], block_index)
                    if "BlockLight" in section:
                        block_info['blockLight'] = nybble(section['BlockLight'], block_index)
                    if "SkyLight" in section:
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

@nbt_child(api_world_index, 'level')
def api_world_level(world):
    return world.world_path / 'level.dat'

@api_world_index.child('player')
def api_world_players_index(world):
    pass

@api_world_players_index.children(wurstmineberg_web.models.Person.from_snowflake_or_wmbid)
def api_world_player(world, player):
    pass

@nbt_child(api_world_player, 'playerdata')
def api_player_data(world, player):
    return world.world_path / 'playerdata' / f'{player.minecraft_uuid}.dat'

@json_child(api_world_player, 'stats')
def api_player_stats(world, player):
    """Returns the player's stats formatted as JSON with stats grouped into objects by category"""
    with (world.world_path / 'stats' / f'{player.minecraft_uuid}.json').open() as stats_file:
        stats = simplejson.load(stats_file, use_decimal=True)
    result = {}
    for stat_name, value in stats.items():
        parent = result
        key_path = stat_name.split('.')
        for key in key_path[:-1]:
            if key not in parent:
                parent[key] = {}
            elif not isinstance(parent[key], dict):
                parent[key] = {'summary': parent[key]}
            parent = parent[key]
        if key_path[-1] in parent:
            parent[key_path[-1]]['summary'] = value
        else:
            parent[key_path[-1]] = value
    return result

@json_child(api_world_index, 'status')
def api_world_status(world):
    return {
        'main': world.is_main,
        'running': world.is_running,
        'version': world.version,
        'list': [person.snowflake_or_wmbid for person in world.online_players]
    }
