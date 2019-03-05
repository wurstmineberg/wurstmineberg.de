import contextlib
import json
from sqlalchemy import create_engine
from sqlalchemy.orm import scoped_session, sessionmaker
from sqlalchemy.ext.declarative import declarative_base

from wurstmineberg_web import app

DEFAULT_DB_CONFIG = {
    'connectionstring': 'postgresql://localhost/wurstmineberg'
}

def get_db_config(config_filename='/opt/wurstmineberg/config/database.json'):
    config = DEFAULT_DB_CONFIG.copy()
    with contextlib.suppress(FileNotFoundError):
        with open(config_filename) as cfg_file:
            config.update(json.load(cfg_file))
    return config

engine = create_engine(get_db_config()['connectionstring'], convert_unicode=True)
db_session = scoped_session(sessionmaker(autocommit=False, autoflush=False, bind=engine))

Base = declarative_base()
Base.query = db_session.query_property()

def init_db():
    # import all modules here that might define models so that
    # they will be registered properly on the metadata.  Otherwise
    # you will have to import them first before calling init_db()
    from wurstmineberg_web.models import Person

    Base.metadata.create_all(bind=engine)

@app.teardown_appcontext
def commit_on_success(error=None):
    if error is None:
        db_session.commit()
    else:
        db_session.rollback()
    db_session.remove()
