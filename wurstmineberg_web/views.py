import flask
import flask_login
import flask_view_tree
import requests
import sqlalchemy.orm.exc
import urllib.parse
import wtforms

import wurstmineberg_web
import wurstmineberg_web.auth
import wurstmineberg_web.forms
import wurstmineberg_web.models
import wurstmineberg_web.util
import wurstmineberg_web.wurstminebot

@flask_view_tree.index(wurstmineberg_web.app)
@wurstmineberg_web.util.template()
def index():
    main_world = wurstmineberg_web.models.World()
    if main_world.is_running:
        version = main_world.version
        if version is None:
            version_url = 'https://minecraft.gamepedia.com/Version_history'
        else:
            version_url = 'https://minecraft.gamepedia.com/{}'.format(urllib.parse.quote(version))
        return {
            'running': True,
            'version': version,
            'version_url': version_url,
            'world': main_world
        }
    else:
        return {'running': False}

@index.child('about')
@wurstmineberg_web.util.template()
def about():
    import wurstmineberg_web.api

    try:
        overview = wurstmineberg_web.api.money_overview.raw()
    except requests.HTTPError as e:
        if e.response.status_code == 502:
            overview = None
        else:
            raise
    return {
        'money_config': wurstmineberg_web.app.config['money'],
        'money_overview': overview,
        'worlds': wurstmineberg_web.models.World
    }

@index.child('stats')
@wurstmineberg_web.util.template()
def stats():
    pass

@index.child('people')
@wurstmineberg_web.util.template()
def people():
    people = wurstmineberg_web.models.Person.get_people_ordered_by_status()
    for key in ['founding', 'later', 'former', 'guest', 'invited', 'vetoed']:
        people[key] = people.get(key, [])

    people['guest'].extend(people['invited'])
    people['former'].extend(people['vetoed'])
    return {'people': people}

@people.children(wurstmineberg_web.models.Person.from_snowflake_or_wmbid)
@wurstmineberg_web.util.template()
def profile(person):
    return {'person': person}

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
@wurstmineberg_web.util.template()
def preferences():
    profile_form = wurstmineberg_web.forms.ProfileForm()
    settings_form = wurstmineberg_web.forms.SettingsForm()
    data = flask.g.user.data
    last_data = None
    displayed_tab = flask.request.args.get('tab', 'profile')

    def set_data():
        profile_form.name.data = flask.g.user.display_name
        profile_form.name.description['placeholder'] = flask.g.user.wmbid
        profile_form.description.data = data.get('description', '')
        profile_form.mojira.data = data.get('mojira', '')
        profile_form.twitter.data = data.get('twitter', {}).get('username', '')
        profile_form.website.data = data.get('website', None)
        profile_form.favcolor.color_dict = data.get('favColor', None)

        for field in settings_form:
            option = field.id
            if option in data.get('options', {}):
                settings_form[option].data = data['options'][option]

        last_data = data

    if flask.request.method == 'GET':
        set_data()
    elif flask.request.method == 'POST' and 'save' in flask.request.form:
        submitted_form = flask.request.form['save']
        if submitted_form == 'save-profile' and profile_form.validate():
            if flask.g.user.snowflake is not None and not flask.g.user.is_admin: # wurstminebot does not have permission to rename admins
                wurstmineberg_web.wurstminebot.set_display_name(flask.g.user, profile_form.name.data)
            data['name'] = profile_form.name.data

            if profile_form.description.data and not profile_form.description.data.isspace():
                data['description'] = profile_form.description.data
            else:
                data.pop('description', None)

            if profile_form.mojira.data and not profile_form.mojira.data.isspace():
                data['mojira'] = profile_form.mojira.data
            else:
                data.pop('mojira', None)

            if profile_form.twitter.data and not profile_form.twitter.data.isspace():
                data.setdefault('twitter', {})['username'] = profile_form.twitter.data
            else:
                data.pop('twitter', None)

            if profile_form.website.data and not profile_form.website.data.isspace():
                data['website'] = profile_form.website.data
            else:
                data.pop('website', None)

            color_dict = profile_form.favcolor.color_dict
            if color_dict:
                data['favColor'] = color_dict
            else:
                data.pop('favColor', None)

            flask.g.user.commit_data()
            set_data()
            flask.flash('Successfully saved profile')

        if submitted_form == 'save-settings' and settings_form.validate():
            options = data.setdefault('options', {})
            for field in settings_form:
                if not isinstance(field, wtforms.HiddenField):
                    options[field.id] = field.data
            data['options'] = options
            flask.g.user.commit_data()
            set_data()
            flask.flash('Successfully saved settings')

    return {'profile_form': profile_form, 'settings_form': settings_form, 'displayed_tab': displayed_tab}
