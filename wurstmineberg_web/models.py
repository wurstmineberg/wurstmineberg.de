from wurstmineberg_web import app
from sqlalchemy import Column, Integer, String, Boolean, ForeignKey
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.orm import relationship, backref
from sqlalchemy.orm.attributes import flag_modified
from social.storage.sqlalchemy_orm import \
    SQLAlchemyUserMixin, \
    SQLAlchemyAssociationMixin, \
    SQLAlchemyNonceMixin, \
    BaseSQLAlchemyStorage
from hashlib import md5
from flask import g

import iso8601
import datetime

from .database import Base, db_session

from flask.ext.login import UserMixin

UID_LENGTH = 16

class User(Base, UserMixin):
    __tablename__ = 'users'
    id = Column(Integer, primary_key=True)
    wmbid = Column(String(UID_LENGTH), ForeignKey('people.wmbid'))
    slackid = Column(String(20))
    active = Column(Boolean, default=True)
    person = relationship('Person', uselist=False, backref=backref('user', uselist=False))

    @property
    def is_active(self):
        return self.active


class Person(Base):
    __tablename__ = 'people'
    wmbid = Column(String(UID_LENGTH), primary_key=True)
    data = Column(JSONB)
    version = Column(Integer)

    @classmethod
    def get_person(self, wmbid):
        return db_session.query(Person).filter_by(wmbid = wmbid).first()

    @classmethod
    def get_by_slack_nick(self, slack_nick):
        return Person.query.filter(Person.data[('slack'),('username')].astext == slack_nick).first()

    @classmethod
    def get_by_slack_id(self, slack_id):
        return Person.query.filter(Person.data[('slack'),('id')].astext == slack_id).first()

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

    def commit_data(self):
        flag_modified(self, 'data')
        db_session.commit()

    @property
    def display_name(self):
        if 'name' in self.data:
            return self.data['name']
        return self.wmbid

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
    def website(self):
        return self.data.get('website', None)

    @property
    def wiki(self):
        return self.data.get('wiki', None)

    @property
    def description(self):
        return self.data.get('description', None)

    def playerhead_url(self, size):
        root_url = '' if g.is_dev else 'http://wurstmineberg.de'
        return '{}/assets/img/head/{}/{}.png'.format(root_url, size, self.wmbid)

    def avatar_urls(self, size):
        # custom avatar, saved in /assets
        imageURLs = []
        hiDPIURLs = []
        # gravatar
        if 'gravatar' in self.data and size <= 2048:
            imageURLs.append('http://www.gravatar.com/avatar/{}?d=404&s={}'.format(md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(size)))
            if (size <= 1024):
                hiDPIURLs.append('http://www.gravatar.com/avatar/{}?d=404&s={}'.format(md5(self.data['gravatar'].encode('utf8')).hexdigest(), str(size) * 2))
        # player head
        root_url = '' if g.is_dev else 'http://wurstmineberg.de'
        imageURLs.append(self.playerhead_url(size));
        hiDPIURLs.append(self.playerhead_url(size * 2));
        # TODO do something with the hiDPI images
        return (imageURLs[0], hiDPIURLs[0])

# We don't need this right now, leave the infrastructure in place if needed later
#class UserTokens(Base):
#    __tablename__ = 'user_tokens'
#    id = Column(Integer, primary_key=True)
#    wmbid = Column(String(16))
#    token = Column(String(200))
