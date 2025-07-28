import flask
import sqlalchemy.orm.exc

import flask_view_tree # https://github.com/fenhl/flask-view-tree

import wurstmineberg_web
import wurstmineberg_web.auth
import wurstmineberg_web.models
import wurstmineberg_web.util

@flask_view_tree.index(wurstmineberg_web.app)
def index():
    raise NotImplementedError('This endpoint has been ported to Rust')

@index.child('stats')
@wurstmineberg_web.util.template()
def stats():
    pass

@index.child('people')
def people():
    raise NotImplementedError('This endpoint has been ported to Rust')

@people.children(wurstmineberg_web.models.Person.from_snowflake_or_wmbid)
def profile(person):
    raise NotImplementedError('This endpoint has been ported to Rust')

@profile.catch_init(sqlalchemy.orm.exc.NoResultFound)
def profile_catch_not_found(exc, value):
    return wurstmineberg_web.util.render_template('invalid-profile', user_id=value, well_formed=True), 404

@profile.catch_init(ValueError)
def profile_catch_value_error(exc, value):
    return wurstmineberg_web.util.render_template('invalid-profile', user_id=value, well_formed=False), 404

@profile.child('reset-key')
def reset_api_key(person):
    if flask.g.user.is_admin or flask.g.user == person:
        del person.api_key
        return flask.redirect(flask.url_for('api_index'))
    else:
        flask.flash(jinja2.Markup("You are not authorized to regenerate {}'s API key.".format(person.__html__())), 'error')
        return flask.redirect(flask.url_for('api_index'))

@index.redirect('profile', decorators=[wurstmineberg_web.auth.member_required])
def get_profile():
    return people, flask.g.user

@index.child('preferences', methods=['GET', 'POST'], decorators=[wurstmineberg_web.auth.member_required])
def preferences():
    raise NotImplementedError('This endpoint has been ported to Rust')
