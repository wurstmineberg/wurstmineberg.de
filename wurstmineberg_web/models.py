import datetime
from flask import g
import flask_login
from hashlib import md5
import iso8601
from sqlalchemy import Column, BigInteger, Integer, String, Boolean, ForeignKey
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.orm import relationship, backref
from sqlalchemy.orm.attributes import flag_modified

from wurstmineberg_web.database import Base, db_session

UID_LENGTH = 16

class Person(Base, flask_login.UserMixin):
    __tablename__ = 'people'

    id = Column(Integer, primary_key=True)
    wmbid = Column(String(UID_LENGTH))
    snowflake = Column(BigInteger)
    active = Column(Boolean, default=True)
    data = Column(JSONB)
    version = Column(Integer)

    def __new__(cls, *, snowflake=None, wmbid=None):
        if (snowflake is None) == (wmbid is None):
            raise ValueError('Must specify exactly one of snowflake or wmbid')
        if snowflake is not None:
            return cls.query.filter_by(snowflake=snowflake).one()
        if wmbid is not None:
            return cls.query.filter_by(wmbid=wmbid).one()

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

    def get_id(self): # required by flask_login
        return self.snowflake

    def is_active(self):
        return self.active

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
        raise NotImplementedError() #TODO get Discord nick

    @property
    def description(self):
        return self.data.get('description', '')

    @property
    def minecraft_name(self):
        if 'minecraft' in self.data and 'nicks' in self.data['minecraft']:
            return self.data['minecraft']['nicks'][-1]

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

    def playerhead_url(self, size):
        return '//api.{}/v2/player/{}/skin/render/head/{}.png'.format(g.host, self.wmbid, size) #TODO update to API v3

    def avatar(self, size):
        imageURLs = []
        hiDPIURLs = []
        # gravatar
        if 'gravatar' in self.data:
            return {
                'url': 'https://www.gravatar.com/avatar/{}?d=404&s={}'.format(md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(min(size, 2048))),
                'hiDPI': 'https://www.gravatar.com/avatar/{}?d=404&s={}'.format(md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(min(size * 2, 2048))),
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
            'url': '{}/img/grid-unknown.png'.format(g.assetserver),
            'hiDPI': '{}/img/grid-unknown.png'.format(g.assetserver),
            'pixelate': True
        }

    @property
    def has_avatar(self):
        return self.data.get('gravatar', None) is not None or self.data.get('minecraft') is not None #TODO check Discord
