import contextlib
import datetime
import enum
import random
import re
import string
import subprocess
import uuid

import flask # PyPI: Flask
import flask_login # PyPI: Flask-Login
import iso8601 # PyPI: iso8601
import jinja2 # PyPI: Jinja2
import mcstatus # PyPI: mcstatus
import pytz # PyPI: pytz
import sqlalchemy # PyPI: SQLAlchemy
import sqlalchemy.dialects.postgresql # PyPI: SQLAlchemy
import sqlalchemy.orm.attributes # PyPI: SQLAlchemy

import people # https://github.com/wurstmineberg/people
import wurstminebot # https://github.com/wurstmineberg/wurstminebot-discord

import wurstmineberg_web
import wurstmineberg_web.util

ADMIN_ROLE_ID = 88329417788502016
API_KEY_LENGTH = 25
UID_LENGTH = 16
WMBID_REGEX = re.compile('[a-z][0-9a-z]{1,15}')

@enum.unique
class Dimension(enum.Enum):
    OVERWORLD = 0
    NETHER = -1
    END = 1

    @classmethod
    def from_url_part(cls, url_part):
        return cls[url_part.upper()]

    @property
    def url_part(self):
        return self.name.lower()

class Person(wurstmineberg_web.db.Model, flask_login.UserMixin):
    __tablename__ = 'people'

    id = sqlalchemy.Column(sqlalchemy.Integer, primary_key=True)
    wmbid = sqlalchemy.Column(sqlalchemy.String(UID_LENGTH))
    snowflake = sqlalchemy.Column(sqlalchemy.BigInteger)
    active = sqlalchemy.Column(sqlalchemy.Boolean, default=True)
    data = sqlalchemy.Column(sqlalchemy.dialects.postgresql.JSONB)
    version = sqlalchemy.Column(sqlalchemy.Integer, default=3)
    apikey = sqlalchemy.Column(sqlalchemy.String(API_KEY_LENGTH))
    discorddata = sqlalchemy.Column(sqlalchemy.dialects.postgresql.JSONB)

    @classmethod
    def from_api_key(cls, key=None, *, exclude=None):
        if exclude is None:
            exclude = set()
        if key is None:
            auth = flask.request.authorization
            if auth and auth.username.strip().lower() == 'api':
                key = auth.password.strip().lower()
        for person in cls.query.all():
            if person in exclude:
                continue
            if key == person.api_key_inner(exclude=exclude):
                return person

    @classmethod
    def from_minecraft_uuid(cls, mc_uuid):
        for person in cls.query.all():
            if person.minecraft_uuid == mc_uuid:
                return person

    @classmethod
    def from_snowflake(cls, snowflake):
        return cls.query.filter_by(snowflake=snowflake).one()

    @classmethod
    def from_wmbid(cls, wmbid):
        return cls.query.filter_by(wmbid=wmbid).one()

    @classmethod
    def from_snowflake_or_wmbid(cls, wmbid_or_snowflake):
        if isinstance(wmbid_or_snowflake, int):
            return cls.from_snowflake(wmbid_or_snowflake)
        elif WMBID_REGEX.fullmatch(wmbid_or_snowflake):
            return cls.from_wmbid(wmbid_or_snowflake)
        else:
            return cls.from_snowflake(int(wmbid_or_snowflake))

    @classmethod
    def from_tag(cls, username, discrim):
        if discrim is None:
            with contextlib.suppress(sqlalchemy.orm.exc.NoResultFound):
                return cls.query.filter_by(wmbid=username).one()
        else:
            for person in cls.query.all():
                if person.discorddata is not None and username == person.discorddata['username'] and discrim == person.discorddata['discriminator']:
                    return person

    @classmethod
    def obj_dump(cls, version=3):
        # use v3 as base as the db probably has v3 anyways
        obj = {'version': 3, 'people': {}}
        for person in cls.query.all():
            person_id = str(person.snowflake_or_wmbid)
            converter = people.PersonConverter(person_id, person.data, person.version)
            obj['people'][person_id] = converter.get_version(3)
        # now for converting everything for realsies
        return people.PeopleConverter(obj).get_version(version)

    @classmethod
    def sorted_people(cls, people):
        def sort_date(person):
            for hist in person.data.get('statusHistory', []):
                if 'date' in hist:
                    date = iso8601.parse_date(hist['date'])
                    if not date.tzinfo:
                        date = date.replace(tzinfo=datetime.timezone.utc)
                    return date
            return datetime.datetime.utcnow().replace(tzinfo=datetime.timezone.utc)

        return sorted(people, key=sort_date)

    @classmethod
    def get_people_ordered_by_status(cls):
        ppl = {}
        for person in cls.query.all():
            history = person.data.get('statusHistory', None)
            if history and len(history) >= 1:
                lasthistory = history[-1]
                status = lasthistory['status']
                try:
                    ppl[status].append(person)
                except KeyError:
                    ppl[status] = [person]
        for status, l in ppl.items():
            ppl[status] = cls.sorted_people(l)
        return ppl

    def __html__(self):
        return jinja2.Markup('<a title="{}" href="{}">@{}</a>'.format(self, self.profile_url, jinja2.escape(self.name)))

    def __repr__(self):
        if self.snowflake is None:
            return 'wurstmineberg_web.models.Person.from_wmbid({!r})'.format(self.wmbid)
        else:
            return 'wurstmineberg_web.models.Person.from_snowflake({!r})'.format(self.snowflake)

    def __str__(self):
        try:
            return self.name
        except Exception:
            return repr(self)

    @property
    def api_key(self):
        return self.api_key_inner()

    def api_key_inner(self, *, exclude=None):
        if exclude is None:
            exclude = set()
        if self.apikey is None:
            new_key = None
            while new_key is None or self.__class__.from_api_key(new_key, exclude=exclude | {self}) is not None: # to avoid duplicates
                new_key = ''.join(random.choice(string.ascii_lowercase + string.digits) for i in range(API_KEY_LENGTH))
            self.apikey = new_key
            wurstmineberg_web.db.session.commit()
        return self.apikey

    @api_key.deleter
    def api_key(self):
        self.apikey = None
        wurstmineberg_web.db.session.commit()

    @property
    def avatar(self):
        available = []
        # Discord avatar
        if self.discorddata is not None and self.discorddata['avatar'] is not None:
            available.append({
                'url': self.discorddata['avatar'],
                'pixelate': False
            })
        # player head
        if self.minecraft_name is not None:
            available.append({
                'url': self.playerhead_url,
                'pixelate': True
            })
        # placeholder
        available.append({
            'url': '{}/img/grid-unknown.png'.format(flask.g.assetserver),
            'pixelate': True
        })
        # API v3 format
        return {**available[0], 'fallbacks': available[1:]}

    def commit_data(self):
        sqlalchemy.orm.attributes.flag_modified(self, 'data')
        wurstmineberg_web.db.session.commit()

    @property
    def description(self):
        return self.data.get('description', '')

    def get_id(self): # required by flask_login
        return self.snowflake

    def is_active(self):
        return self.active

    @property
    def is_admin(self):
        return self.discorddata is not None and str(ADMIN_ROLE_ID) in self.discorddata['roles']

    @property
    def join_date(self):
        for hist in self.data.get('statusHistory', []):
            if hist['status'] == 'later' and 'date' in hist:
                date = iso8601.parse_date(hist['date'])
                if not date.tzinfo:
                    date = date.replace(tzinfo=datetime.timezone.utc)
                return date

    @property
    def mention(self):
        if self.snowflake is None:
            return wurstminebot.escape(self.wmbid)
        else:
            return f'<@{self.snowflake}>'

    @property
    def minecraft_name(self):
        if 'minecraft' in self.data and 'nicks' in self.data['minecraft']:
            return self.data['minecraft']['nicks'][-1]

    @property
    def minecraft_uuid(self):
        if 'minecraft' in self.data:
            return uuid.UUID(self.data['minecraft']['uuid'])

    @property
    def mojira(self):
        return self.data.get('mojira', None)

    @property
    def name(self):
        if self.discorddata is not None:
            if self.discorddata['nick'] is not None:
                return self.discorddata['nick']
            return self.discorddata['username']
        if 'name' in self.data:
            return self.data['name']
        if self.wmbid is not None:
            return self.wmbid
        if len(self.data.get('minecraft', {}).get('nicks', [])) > 0:
            return self.data['minecraft']['nicks'][0]
        #TODO get from Minecraft UUID
        raise ValueError(f'{self!r} has no name')

    def option(self, option_name):
        default_true_options = {'activity_tweets', 'chatsync_highlight', 'inactivity_tweets'} # These options are on by default. All other options are off by default.
        return self.data.get('options', {}).get(option_name, option_name in default_true_options)

    @property
    def playerhead_url(self):
        return flask.url_for('api_player_head', person=str(self.snowflake_or_wmbid), _external=True)

    @property
    def profile_url(self):
        return flask.url_for('profile', person=str(self.snowflake_or_wmbid), _external=True)

    @property
    def snowflake_or_wmbid(self):
        if self.snowflake is None:
            return self.wmbid
        else:
            return self.snowflake

    @property
    def timezone(self):
        if 'timezone' in self.data:
            return pytz.timezone(self.userdata['timezone'])

    @property
    def twitch(self):
        return self.data.get('twitch', None)

    @twitch.setter
    def twitch(self, value):
        """The value should be structured as in https://dev.twitch.tv/docs/api/reference#get-users"""
        self.data['twitch'] = value
        self.commit_data()

    @property
    def twitter_name(self):
        twitter = self.data.get('twitter', None)
        if twitter:
            return twitter.get('username', None)

    @property
    def url_part(self):
        return str(self.snowflake_or_wmbid)

    @property
    def website(self):
        return self.data.get('website', None)

    @property
    def wiki(self):
        return self.data.get('wiki', None)

class WorldMeta(type):
    def __iter__(self):
        for world_path in sorted((wurstmineberg_web.util.BASE_PATH / 'world').iterdir()):
            if world_path.is_dir():
                yield World(world_path.name)

class World(metaclass=WorldMeta):
    def __init__(self, name='wurstmineberg'): #TODO get default from config
        self.name = name #TODO check if world exists

    def __repr__(self):
        return 'wurstmineberg_web.models.World({!r})'.format(self.name)

    def __str__(self):
        return self.name

    @property
    def dir(self):
        return wurstmineberg_web.util.BASE_PATH / 'world' / self.name

    @property
    def is_main(self):
        return self.name == 'wurstmineberg' #TODO get from config

    @property
    def is_running(self):
        return subprocess.run(['systemctl', 'is-active', 'minecraft@{}.service'.format(self)]).returncode == 0

    @property
    def online_players(self):
        server = mcstatus.MinecraftServer.lookup('wurstmineberg.de' if self.is_main else '{}.wurstmineberg.de'.format(self))
        try:
            status = server.status()
        except ConnectionRefusedError:
            return []
        except (BrokenPipeError, ConnectionResetError):
            # try again
            try:
                status = server.status()
            except ConnectionRefusedError:
                return []
        return [
            Person.from_minecraft_uuid(uuid.UUID(player.id))
            for player in (status.players.sample or [])
        ]

    def player_data(self, player):
        import wurstmineberg_web.api

        return wurstmineberg_web.api.api_player_data(self, player)

    def region_path(self, dimension):
        return self.world_path / {
            Dimension.OVERWORLD: 'region',
            Dimension.NETHER: 'DIM-1/region',
            Dimension.END: 'DIM1/region'
        }[dimension]

    @property
    def version(self):
        if not (self.dir / 'minecraft_server.jar').exists():
            return None
        #TODO return None for custom/modded servers
        return (self.dir / 'minecraft_server.jar').resolve().stem[len('minecraft_server.'):]

    @property
    def world_path(self):
        return self.dir / 'world' #TODO check server.properties
