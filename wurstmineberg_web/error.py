from wurstmineberg_web import app

from social.exceptions import SocialAuthBaseException, AuthFailed
from flask import render_template, flash, Markup

@app.errorhandler(403)
@app.errorhandler(404)
@app.errorhandler(410)
@app.errorhandler(500)
def error_handler(error):
    if isinstance(error, SocialAuthBaseException):
        flash(Markup.escape(str(error)), 'login_error')
        return render_template('login.html')
    try:
        code = error.code
    except AttributeError:
        code = 500
    report = code == 500
    flash(Markup.escape(str(error)), 'error')
    return render_template('error.html', error=error, report=report), code
