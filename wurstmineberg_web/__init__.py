#!/usr/bin/env python3
"""
Wurstmineberg website
"""

import flask # PyPI: Flask
import flask_bootstrap # PyPI: Flask-Bootstrap
import flask_pagedown # PyPI: Flask-PageDown
import flask_sqlalchemy # Flask-SQLAlchemy
import flaskext.markdown # PyPI: Flask-Markdown
import jinja2 # PyPI: jinja2
import pymdownx.emoji # PyPI: pymdown-extensions
import pymdownx.extra # PyPI: pymdown-extensions
import pymdownx.tilde # PyPI: pymdown-extensions

import flask_wiki # https://github.com/fenhl/flask-wiki

import wurstmineberg_web.util

app = None
db = None

def create_app(production):
    global app
    global db
    global wurstmineberg_web

    app = flask.Flask(__name__, template_folder='templates/')

    app.url_map.strict_slashes = False
    app.jinja_env.autoescape = jinja2.select_autoescape(
        default_for_string=True,
        enabled_extensions=('html', 'xml', 'j2')
    )
    # load config
    app.config['SQLALCHEMY_DATABASE_URI'] = 'postgresql:///wurstmineberg'
    if wurstmineberg_web.util.CONFIG_PATH.exists():
        app.config.update(wurstmineberg_web.util.load_json(wurstmineberg_web.util.CONFIG_PATH))

    # set up database
    db = flask_sqlalchemy.SQLAlchemy(app)
    # load Python modules
    import wurstmineberg_web.views
    import wurstmineberg_web.auth
    import wurstmineberg_web.api
    import wurstmineberg_web.error
    import wurstmineberg_web.models
    import wurstmineberg_web.wiki
    # set up Bootstrap
    flask_bootstrap.Bootstrap(app)
    # set up Markdown and wiki
    md = flaskext.markdown.Markdown(app, extensions=['toc'], extension_configs={
        'toc': {
            'marker': ''
        }
    })
    md.register_extension(wurstmineberg_web.wiki.WmbidMentionExtension)
    emoji_ext = pymdownx.emoji.EmojiExtension()
    emoji_ext.setConfig('emoji_generator', pymdownx.emoji.to_alt)
    emoji_ext.setConfig('emoji_index', pymdownx.emoji.twemoji)
    md._instance.registerExtensions([emoji_ext], {})
    md.register_extension(pymdownx.extra.ExtraExtension)
    md.register_extension(pymdownx.tilde.DeleteSubExtension)
    flask_wiki.child(
        wurstmineberg_web.views.index,
        db=db,
        edit_decorators=[wurstmineberg_web.auth.member_required],
        md=md,
        mentions_to_tags=wurstmineberg_web.wiki.mentions_to_tags,
        save_hook=wurstmineberg_web.wiki.save_hook,
        tags_to_mentions=wurstmineberg_web.wiki.tags_to_mentions,
        user_class=wurstmineberg_web.models.Person,
        user_class_constructor=wurstmineberg_web.models.Person.from_snowflake_or_wmbid,
        wiki_name='Wurstmineberg Wiki'
    )
    # set up Markdown preview
    flask_pagedown.PageDown(app)

    if not production:
        import os
        # Because of bugs https://gist.github.com/uniphil/7777590 we need to use absolute paths
        @app.route('/assetserver/<path:path>')
        def serve_assetserver(path):
            return flask.send_from_directory(os.path.join(app.root_path, 'assetserver'), path)

    @app.before_request
    def before_request():
        flask.g.is_dev = uwsgi_options.get('is_dev', False)

        # Template variables
        flask.g.host = 'dev.wurstmineberg.de' if flask.g.is_dev else 'wurstmineberg.de'

        if production:
            flask.g.assetserver = 'https://assets.' + flask.g.host
        else:
            flask.g.assetserver = '/assetserver'

    wurstmineberg_web.auth.setup(app)

    return app


try:
    import uwsgi
    uwsgi_options = uwsgi.opt
    has_uwsgi = True
except ImportError:
    uwsgi_options = {}
    has_uwsgi = False
