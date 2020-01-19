import flask
import io
import subprocess
import traceback

from wurstmineberg_web import app
import wurstmineberg_web.util

CRASH_NOTICE = """To: root@wurstmineberg.de
From: {whoami}@{hostname}
Subject: wurstmineberg.de internal server error

An internal server error occurred on wurstmineberg.de.
User: {user}
URL: {url}
"""

def notify_crash(exc=None):
    whoami = subprocess.run(['whoami'], stdout=subprocess.PIPE, check=True).stdout.decode('utf-8').strip()
    hostname = subprocess.run(['hostname', '-f'], stdout=subprocess.PIPE, check=True).stdout.decode('utf-8').strip()
    try:
        user = str(flask.g.user)
    except Exception:
        user = None
    try:
        url = str(flask.g.view_node.url)
    except Exception:
        url = None
    mail_text = CRASH_NOTICE.format(whoami=whoami, hostname=hostname, user=user, url=url)
    if exc is not None:
        mail_text += '\n' + traceback.format_exc()
    return subprocess.run(['ssmtp', 'root@wurstmineberg.de'], input=mail_text.encode('utf-8'), check=True)

@app.errorhandler(403)
@app.errorhandler(404)
@app.errorhandler(410)
@app.errorhandler(500)
def error_handler(error):
    try:
        code = error.code
    except AttributeError:
        code = 500
    report = code == 500
    reported = False
    if report:
        try:
            notify_crash(error)
        except Exception:
            pass
        else:
            reported = True
    return wurstmineberg_web.util.render_template('error', error=error, is_exception=lambda v: isinstance(v, Exception), report=report, reported=reported, traceback=traceback), code
