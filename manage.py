#!/usr/bin/env python
import sys

from flask.ext.script import Server, Manager, Shell

sys.path.append('..')

from wurstmineberg_web import create_app
app = create_app(False)

from wurstmineberg_web.database import db_session, engine


manager = Manager(app)
manager.add_command('runserver', Server())
manager.add_command('shell', Shell(make_context=lambda: {
    'app': app,
    'db_session': db_session
}))


@manager.command
def syncdb():
    from wurstmineberg_web.database import Base
    from social.apps.flask_app.default import models
    Base.metadata.create_all(engine)
    models.PSABase.metadata.create_all(engine)

if __name__ == '__main__':
    manager.run()
