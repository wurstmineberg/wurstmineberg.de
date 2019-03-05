import flask
import io
import traceback

from wurstmineberg_web import app

@app.errorhandler(403)
@app.errorhandler(404)
@app.errorhandler(410)
@app.errorhandler(500)
def error_handler(error):
    if isinstance(error, SocialAuthBaseException):
        flask.flash(flask.Markup.escape(str(error)), 'login_error')
        return flask.render_template('login.html')
    try:
        code = error.code
    except AttributeError:
        code = 500
    report = code == 500
    flask.flash(flask.Markup.escape(str(error)), 'error')
    return flask.render_template('error.html', error=error, is_exception=lambda v: isinstance(v, Exception), report=report, traceback=traceback), code
