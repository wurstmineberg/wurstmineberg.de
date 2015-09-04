from wurstmineberg_web import app
from sqlalchemy import Column, Integer, String, Boolean
from sqlalchemy.dialects.postgresql import JSONB
from sqlalchemy.orm.attributes import flag_modified
from social.storage.sqlalchemy_orm import \
    SQLAlchemyUserMixin, \
    SQLAlchemyAssociationMixin, \
    SQLAlchemyNonceMixin, \
    BaseSQLAlchemyStorage

from .database import Base, db_session

from flask.ext.login import UserMixin

UID_LENGTH = 16

class User(Base, UserMixin):
    __tablename__ = 'users'
    id = Column(Integer, primary_key=True)
    wmbid = Column(String(UID_LENGTH))
    slackid = Column(String(20))
    active = Column(Boolean, default=True)

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

    def commit_data(self):
        flag_modified(self, 'data')
        db_session.commit()


# We don't need this right now, leave the infrastructure in place if needed later
#class UserTokens(Base):
#    __tablename__ = 'user_tokens'
#    id = Column(Integer, primary_key=True)
#    wmbid = Column(String(16))
#    token = Column(String(200))
