import datetime
import flask
import flask_login
import iso8601
import jinja2
import people
import random
import re
from sqlalchemy import Column, BigInteger, Integer, String, Boolean, ForeignKey
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.orm import relationship, backref
from sqlalchemy.orm.attributes import flag_modified
import string
import subprocess

import wurstmineberg_web.database
import wurstmineberg_web.util

ADMIN_ROLE_ID = 88329417788502016
API_KEY_LENGTH = 25
UID_LENGTH = 16
WMBID_REGEX = '[a-z][0-9a-z]{1,15}'

class Person(wurstmineberg_web.database.Base, flask_login.UserMixin):
    __tablename__ = 'people'

    id = Column(Integer, primary_key=True)
    wmbid = Column(String(UID_LENGTH))
    snowflake = Column(BigInteger)
    active = Column(Boolean, default=True)
    data = Column(JSONB)
    version = Column(Integer, default=3)
    apikey = Column(String(API_KEY_LENGTH))
    discorddata = Column(JSONB)

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
    def from_snowflake(cls, snowflake):
        return cls.query.filter_by(snowflake=snowflake).one()

    @classmethod
    def from_wmbid(cls, wmbid):
        return cls.query.filter_by(wmbid=wmbid).one()

    @classmethod
    def from_snowflake_or_wmbid(cls, wmbid_or_snowflake):
        if re.fullmatch(WMBID_REGEX, wmbid_or_snowflake):
            return cls.from_wmbid(wmbid_or_snowflake)
        else:
            return cls.from_snowflake(int(wmbid_or_snowflake))

    @classmethod
    def from_tag(cls, username, discrim):
        if discrim is None:
            return cls.query.filter_by(wmbid=username).one()
        else:
            for person in cls.query.all():
                if username == person.discorddata['username'] and discrim == person.discorddata['discriminator']:
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
        return jinja2.Markup('<a title="{}" href="{}">@{}</a>'.format(self, self.profile_url, jinja2.escape(self.display_name)))

    def __repr__(self):
        if self.snowflake is None:
            return 'wurstmineberg_web.models.Person.from_wmbid({!r})'.format(self.wmbid)
        else:
            return 'wurstmineberg_web.models.Person.from_snowflake({!r})'.format(self.snowflake)

    def __str__(self):
        try:
            return self.display_name
        except Exception:
            return repr(self)

    def get_id(self): # required by flask_login
        return self.snowflake

    def is_active(self):
        return self.active

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
            wurstmineberg_web.database.db_session.commit()
        return self.apikey

    @api_key.deleter
    def api_key(self):
        self.apikey = None
        wurstmineberg_web.database.db_session.commit()

    @property
    def is_admin(self):
        return self.discorddata is not None and ADMIN_ROLE_ID in self.discorddata['roles']

    def commit_data(self):
        flag_modified(self, 'data')
        wurstmineberg_web.database.db_session.commit()

    @property
    def display_name(self):
        if self.discorddata is not None:
            if self.discorddata['nick'] is not None:
                return self.discorddata['nick']
            return self.discorddata['username']
        if 'name' in self.data:
            return self.data['name']
        if self.wmbid is not None:
            return self.wmbid

    @property
    def description(self):
        return self.data.get('description', '')

    @property
    def minecraft_name(self):
        if 'minecraft' in self.data and 'nicks' in self.data['minecraft']:
            return self.data['minecraft']['nicks'][-1]

    @property
    def snowflake_or_wmbid(self):
        if self.snowflake is None:
            return self.wmbid
        else:
            return self.snowflake

    @property
    def url_part(self):
        return str(self.snowflake_or_wmbid)

    @property
    def twitter_name(self):
        twitter = self.data.get('twitter', None)
        if twitter:
            return twitter.get('username', None)

    @property
    def mojira(self):
        return self.data.get('mojira', None)

    @property
    def website(self):
        return self.data.get('website', None)

    @property
    def wiki(self):
        return self.data.get('wiki', None)

    @property
    def profile_url(self):
        flask.url_for('profile', person=str(self.snowflake_or_wmbid))

    def playerhead_url(self, size):
        return '//api.{}/v2/player/{}/skin/render/head/{}.png'.format(flask.g.host, self.wmbid, size) #TODO update to API v3

    def avatar(self, size):
        # Discord avatar
        if self.discorddata is not None and self.discorddata['avatar'] is not None:
            return {
                'url': self.discorddata['avatar'],
                'hiDPI': self.discorddata['avatar'],
                'pixelate': False
            }
        # player head
        if self.minecraft_name is not None:
            return {
                'url': self.playerhead_url(min(size, 1024)),
                'hiDPI': self.playerhead_url(min(size * 2, 1024)),
                'pixelate': True
            }
        # placeholder
        return {
            'url': '{}/img/grid-unknown.png'.format(flask.g.assetserver),
            'hiDPI': '{}/img/grid-unknown.png'.format(flask.g.assetserver),
            'pixelate': True
        }

class World:
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
    def version(self):
        if not (self.dir / 'minecraft_server.jar').exists():
            return None
        #TODO return None for custom/modded servers
        return (self.dir / 'minecraft_server.jar').resolve().stem[len('minecraft_server.'):]
