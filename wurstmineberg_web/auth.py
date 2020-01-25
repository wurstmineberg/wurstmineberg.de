import functools
import urllib.parse

import flask # PyPI: Flask
import flask_dance.consumer # PyPI: Flask-Dance
import flask_dance.contrib.discord # PyPI: Flask-Dance
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

def is_safe_url(target):
    ref_url = urllib.parse.urlparse(flask.request.host_url)
    test_url = urllib.parse.urlparse(urllib.parse.urljoin(flask.request.host_url, target))
    return test_url.scheme in ('http', 'https') and ref_url.netloc == test_url.netloc

def member_required(f):
    @functools.wraps(f)
    def wrapper(*args, **kwargs):
        if not flask.g.user.is_active:
            return flask.make_response(("You don't have permission to access this page because you're not a server member.", 403, [])) #TODO template
        return f(*args, **kwargs)

    return flask_login.login_required(wrapper)

def setup(app):
    if 'clientID' not in app.config.get('wurstminebot', {}) or 'clientSecret' not in app.config.get('wurstminebot', {}):
        return #TODO mount error messages at /login and /auth
    app.config['SECRET_KEY'] = app.config['wurstminebot']['clientSecret']
    app.config['USE_SESSION_FOR_NEXT'] = True

    app.register_blueprint(flask_dance.contrib.discord.make_discord_blueprint(
        client_id=app.config['wurstminebot']['clientID'],
        client_secret=app.config['wurstminebot']['clientSecret'],
        scope='identify',
        redirect_to='auth_callback'
    ), url_prefix='/login')

    twitch_blueprint = flask_dance.consumer.OAuth2ConsumerBlueprint(
        'twitch', __name__,
        client_id=app.config['twitch']['clientID'],
        client_secret=app.config['twitch']['clientSecret'],
        base_url='https://api.twitch.tv/helix/',
        token_url='https://id.twitch.tv/oauth2/token',
        authorization_url='https://id.twitch.tv/oauth2/authorize',
        redirect_to='twitch_auth_callback'
    )
    app.register_blueprint(twitch_blueprint, url_prefix='/login')

    login_manager = flask_login.LoginManager()
    login_manager.login_view = 'discord.login'
    login_manager.login_message = None # Because discord.login does not show flashes, any login message would be shown after a successful login. This would be confusing.
    login_manager.anonymous_user = AnonymousUser

    @login_manager.user_loader
    def load_user(user_id):
        try:
            return Person.from_snowflake(user_id)
        except (TypeError, ValueError):
            return None

    login_manager.init_app(app)

    @app.before_request
    def global_user():
        if flask_login.current_user.is_admin and 'viewAs' in app.config.get('web', {}):
            flask.g.view_as = True
            flask.g.user = Person.from_snowflake(app.config['web']['viewAs'])
        else:
            flask.g.view_as = False
            flask.g.user = flask_login.current_user

    @app.context_processor
    def inject_user():
        try:
            return {'user': flask.g.user}
        except AttributeError:
            return {'user': None}

    @app.route('/auth')
    def auth_callback():
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
        flask_login.login_user(person, remember=True)
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
        if not twitch_blueprint.session.authorized:
            flask.flash('Twitch login failed.', 'error')
            return flask.redirect(flask.url_for('index'))
        response = twitch_blueprint.session.get('users')
        if not response.ok:
            return flask.make_response(('Discord returned error {} at {}: {}'.format(response.status_code, jinja2.escape(response.url), jinja2.escape(response.text)), response.status_code, []))
        if flask.g.user.is_active:
            flask.g.user.twitch = response.json()['data'][0]
        else:
            flask.flash('Please sign in via Discord before linking your Twitch account.', 'error')
            return flask.redirect(flask.url_for('index'))
        next_url = flask.session.get('next')
        if next_url is None:
            return flask.redirect(flask.url_for('index'))
        elif is_safe_url(next_url):
            return flask.redirect(next_url)
        else:
            return flask.abort(400)

    @app.route('/logout')
    def logout():
        flask_login.logout_user()
        return flask.redirect(flask.url_for('index'))
