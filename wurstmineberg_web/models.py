import datetime
import flask
import flask_login
import hashlib
import iso8601
import jinja2
import random
import re
from sqlalchemy import Column, BigInteger, Integer, String, Boolean, ForeignKey
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.orm import relationship, backref
from sqlalchemy.orm.attributes import flag_modified
import string

from wurstmineberg_web.database import Base, db_session

API_KEY_LENGTH = 25
UID_LENGTH = 16

class Person(Base, flask_login.UserMixin):
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
        if re.fullmatch('[a-z][0-9a-z]{1,15}', wmbid_or_snowflake):
            return cls.from_wmbid(wmbid_or_snowflake)
        else:
            return cls.from_snowflake(int(wmbid_or_snowflake))

    @classmethod
    def sorted_people(self, people):
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
    def get_people_ordered_by_status(self):
        ppl = {}
        for person in Person.query.all():
            history = person.data.get('statusHistory', None)
            if history and len(history) >= 1:
                lasthistory = history[-1]
                status = lasthistory['status']
                try:
                    ppl[status].append(person)
                except KeyError:
                    ppl[status] = [person]
        for status, l in ppl.items():
            ppl[status] = self.sorted_people(l)
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
            db_session.commit()
        return self.apikey

    @api_key.deleter
    def api_key(self):
        self.apikey = None
        db_session.commit()

    @property
    def is_admin(self):
        return False #TODO

    def commit_data(self):
        flag_modified(self, 'data')
        db_session.commit()

    @property
    def display_name(self):
        if 'name' in self.data:
            return self.data['name']
        if self.wmbid is not None:
            return self.wmbid
        raise NotImplementedError() #TODO get Discord nick first

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
        imageURLs = []
        hiDPIURLs = []
        # gravatar
        if 'gravatar' in self.data:
            return {
                'url': 'https://www.gravatar.com/avatar/{}?d=404&s={}'.format(hashlib.md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(min(size, 2048))),
                'hiDPI': 'https://www.gravatar.com/avatar/{}?d=404&s={}'.format(hashlib.md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(min(size * 2, 2048))),
                'pixelate': False
            }
        #TODO Discord avatar
        # player head
        if self.minecraft_name is not None:
            return {
                'url': self.playerhead_url(min(size, 1024)),
                'hiDPI': self.playerhead_url(min(size * 2, 1024)),
                'pixelate': True
            }
        return {
            'url': '{}/img/grid-unknown.png'.format(flask.g.assetserver),
            'hiDPI': '{}/img/grid-unknown.png'.format(flask.g.assetserver),
            'pixelate': True
        }

    @property
    def has_avatar(self):
        return self.data.get('gravatar', None) is not None or self.data.get('minecraft') is not None #TODO check Discord
