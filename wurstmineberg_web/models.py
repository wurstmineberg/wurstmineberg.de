from wurstmineberg_web import app
from sqlalchemy import Column, Integer, String, Boolean
from social.storage.sqlalchemy_orm import SQLAlchemyUserMixin, \
                                          SQLAlchemyAssociationMixin, \
                                          SQLAlchemyNonceMixin, \
                                          BaseSQLAlchemyStorage

from .database import Base, db_session

from flask.ext.login import UserMixin

UID_LENGTH = 16

class User(Base, UserMixin):
    __tablename__ = 'users'
    id = Column(Integer, primary_key=True)
    wmbid = Column(String(16))
    email = Column(String(200))
    active = Column(Boolean, default=True)

    def is_active(self):
        return self.active

class UserTokens(Base):
    __tablename__ = 'user_tokens'
    id = Column(Integer, primary_key=True)
    wmbid = Column(String(16))
    token = Column(String(200))
