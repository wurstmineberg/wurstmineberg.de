from wurstmineberg_web import app, g
import flask.ext.login as login
from social.apps.flask_app.template_filters import backends
from social.pipeline.partial import partial
from flask import redirect, render_template, request, session
from flask.views import View, MethodView

from .models import User, UserTokens
from .database import db_session

login_manager = login.LoginManager()
login_manager.login_view = 'page.login'
login_manager.login_message = 'Log in plz'
login_manager.init_app(app)

app.config['SOCIAL_AUTH_USER_MODEL'] = 'wurstmineberg_web.models.User'
app.config['SOCIAL_AUTH_FIELDS_STORED_IN_SESSION'] = ['keep']

app.context_processor(backends)

@app.before_request
def global_user():
    g.user = login.current_user

@login_manager.user_loader
def load_user(userid):
    try:
        return User.query.get(userid)
    except (TypeError, ValueError):
        pass

@app.context_processor
def inject_user():
    try:
        return {'user': g.user}
    except AttributeError:
        return {'user': None}

@app.errorhandler(500)
def error_handler(error):
    if isinstance(error, SocialAuthBaseException):
        return redirect('/loginerror')

@partial
def check_token(strategy=None, details=None, user=None, response=None, backend=None, *args, **kwargs):
    if user is None:
        # This user is not yet registered. Verify token.
        if 'initial-user-token' in session and session['initial-user-token']:
            token = session['initial-user-token']
            entry = db_session.query(UserTokens).filter_by(token = token).first()
            if entry:
                wmbid = entry.wmbid
                fields = dict((name, kwargs.get(name) or details.get(name))
                  for name in strategy.setting('USER_FIELDS', ['email']))
                user = strategy.create_user(**fields)
                user.wmbid = wmbid
                return {
                    'user': user,
                    'is_new': True,
                }
            else:
                return redirect('/verify_token/error')
        else:
            return redirect('/verify_token/')
    else:
        return {'is_new': False}
        #return redirect('/complete/' + backend)

class VerifyToken(MethodView):
    def get_template_name(self):
        return 'verify_token.html'

    def get_backend(self):
        return session['partial_pipeline']['backend']

    def get(self, error):
        if 'initial-user-token' in session:
            del session['initial-user-token']
        display_error = error is not None and error == 'error'
        return render_template(self.get_template_name(), display_error=display_error, backend=self.get_backend())

    def post(self, error):
        display_error = error is not None and error == 'error'
        if 'user-token' in request.form:
            token = request.form['user-token']
            session['initial-user-token'] = token
            return redirect('/complete/' + self.get_backend() + '/')
        else:
            return render_template(self.get_template_name(), display_error=True, backend=self.get_backend())


verify_token_view = VerifyToken.as_view('verify_token')
app.add_url_rule('/verify_token/', view_func=verify_token_view, defaults={'error': None})
app.add_url_rule('/verify_token/<error>', view_func=verify_token_view)
