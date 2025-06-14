import functools
import urllib.parse

import flask # PyPI: Flask
import flask_dance.contrib.discord # PyPI: Flask-Dance
import flask_dance.contrib.twitch # PyPI: Flask-Dance
import flask_login # PyPI: Flask-Login
import jinja2 # PyPI: Jinja2
import sqlalchemy.orm.exc # PyPI: SQLAlchemy

from wurstmineberg_web import app
from wurstmineberg_web.models import Person

class AnonymousUser(flask_login.AnonymousUserMixin):
    def __html__(self):
        return jinja2.Markup('<i>anonymous</i>')

    def __str__(self):
        return 'anonymous'

    @property
    def is_admin(self):
        return False

    @property
    def timezone(self):
        return None

def is_safe_url(target):
    ref_url = urllib.parse.urlparse(flask.request.host_url)
    test_url = urllib.parse.urlparse(urllib.parse.urljoin(flask.request.host_url, target))
    return test_url.scheme in ('http', 'https') and ref_url.netloc == test_url.netloc

def member_required(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        if not flask.g.user.is_authenticated:
            return flask.redirect('/login/discord') #TODO redirect_to parameter
        if not flask.g.user.is_active:
            return flask.make_response(("You don't have permission to access this page because you're not a server member.", 403, [])) #TODO template
        return f(*args, **kwargs)

    return wrapper

def setup(app):
    if 'clientSecret' not in app.config.get('wurstminebot', {}):
        return #TODO mount error messages at /login and /auth
    app.config['SECRET_KEY'] = app.config['wurstminebot']['clientSecret']
    app.config['USE_SESSION_FOR_NEXT'] = True

    app.register_blueprint(flask_dance.contrib.twitch.make_twitch_blueprint(
        client_id=app.config['twitch']['clientID'],
        client_secret=app.config['twitch']['clientSecret'],
        redirect_to='twitch_auth_callback'
    ), url_prefix='/login')

    @login_manager.user_loader
    def load_user(user_id):
        try:
            return Person.from_snowflake(user_id)
        except (TypeError, ValueError):
            return None

    @app.before_request
    def global_user():
        if 'x-wurstmineberg-authorized-discord-id' in flask.request.headers:
            flask.g.user = Person.from_snowflake(flask.request.headers['x-wurstmineberg-authorized-discord-id'])
        else:
            flask.g.user = AnonymousUser()

    @app.context_processor
    def inject_user():
        try:
            return {'user': flask.g.user}
        except AttributeError:
            return {'user': None}

    def auth_callback():
        #TODO similar error handling in Rust
        if not flask_dance.contrib.discord.discord.authorized:
            flask.flash('Discord login failed.', 'error')
            return flask.redirect(flask.url_for('index'))
        response = flask_dance.contrib.discord.discord.get('/api/v6/users/@me')
        if not response.ok:
            return flask.make_response(('Discord returned error {} at {}: {}'.format(response.status_code, jinja2.escape(response.url), jinja2.escape(response.text)), response.status_code, []))
        try:
            person = Person.from_snowflake(response.json()['id'])
        except sqlalchemy.orm.exc.NoResultFound:
            flask.flash('You have successfully authenticated your Discord account, but you\'re not in the Wurstmineberg Discord server.', 'error')
            return flask.redirect(flask.url_for('index'))
        if not person.is_active:
            flask.flash('Your account has not yet been whitelisted. Please schedule a server tour in #general.', 'error')
            return flask.redirect(flask.url_for('index'))
        flask.flash(jinja2.Markup('Hello {}.'.format(person.__html__())))
        next_url = flask.session.get('next')
        if next_url is None:
            return flask.redirect(flask.url_for('index'))
        elif is_safe_url(next_url):
            return flask.redirect(next_url)
        else:
            return flask.abort(400)

    @app.route('/auth/twitch')
    def twitch_auth_callback():
        if not flask.g.user.is_active:
            flask.flash('Please sign in via Discord before linking your Twitch account.', 'error')
            return flask.redirect(flask.url_for('index'))
        if not flask_dance.contrib.twitch.twitch.authorized:
            flask.flash('Twitch login failed.', 'error')
            return flask.redirect(flask.url_for('index'))
        response = flask_dance.contrib.twitch.twitch.get('users')
        if not response.ok:
            return flask.make_response(('Twitch returned error {} at {}: {}'.format(response.status_code, jinja2.escape(response.url), jinja2.escape(response.text)), response.status_code, []))
        flask.g.user.twitch = response.json()['data'][0]
        flask.flash('Successfully linked Twitch account.')
        next_url = flask.session.get('next')
        if next_url is None:
            return flask.redirect(flask.url_for('index'))
        elif is_safe_url(next_url):
            return flask.redirect(next_url)
        else:
            return flask.abort(400)
