import traceback

import flask
import requests

import wurstmineberg_web
import wurstmineberg_web.util

CRASH_NOTICE = """An internal server error occurred on wurstmineberg.de.
User: {user}
URL: {url}
{tb}"""

def notify_crash():
    try:
        user = str(flask.g.user)
    except Exception as e:
        user = f'({e})'
    try:
        url = str(flask.g.view_node.url)
    except Exception as e:
        url = f'({e})'
    exc_text = CRASH_NOTICE.format(user=user, url=url, tb=traceback.format_exc())

@wurstmineberg_web.app.errorhandler(403)
@wurstmineberg_web.app.errorhandler(404)
@wurstmineberg_web.app.errorhandler(410)
def error_handler(error):
    try:
        code = error.code
    except AttributeError:
        code = 500
    report = code == 500
    reported = False
    if report:
        try:
            notify_crash()
        except Exception:
            traceback.print_exc()
        else:
            reported = True
    return wurstmineberg_web.util.render_template('error', error=error, is_exception=lambda v: isinstance(v, Exception), report=report, reported=reported, traceback=traceback), code

@wurstmineberg_web.app.errorhandler(500)
def internal_server_error(error):
    try:
        user = str(flask.g.user)
    except Exception as e:
        user = f'({e})'
    try:
        url = str(flask.g.view_node.url)
    except Exception as e:
        url = f'({e})'
    return flask.Response(CRASH_NOTICE.format(user=user, url=url, tb=traceback.format_exc()), mimetype='text/plain'), 500
