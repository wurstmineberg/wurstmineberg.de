import flask
import flask_login
import flask_view_tree
import sqlalchemy.orm.exc
import wtforms

import wurstmineberg_web
import wurstmineberg_web.auth
import wurstmineberg_web.forms
from wurstmineberg_web.models import Person
from wurstmineberg_web.util import templated

@flask_view_tree.index(wurstmineberg_web.app)
@templated()
def index():
    pass

@index.child('about')
@templated()
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
        'money_overview': overview
    }

@index.child('stats')
@templated()
def stats():
    pass

@index.child('people')
@templated()
def people():
    people = Person.get_people_ordered_by_status()
    for key in ['founding', 'later', 'former', 'guest', 'invited', 'vetoed']:
        people[key] = people.get(key, [])

    people['guest'].extend(people['invited'])
    people['former'].extend(people['vetoed'])
    return {'people': people}

@people.children(Person.from_snowflake_or_wmbid)
@templated()
def profile(person):
    return {'person': person}

@profile.catch_init(sqlalchemy.orm.exc.NoResultFound)
def profile_catch_init(exc, value):
    return flask.abort(404)

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
@templated()
def preferences():
    profile_form = wurstmineberg_web.forms.ProfileForm()
    settings_form = wurstmineberg_web.forms.SettingsForm()
    data = flask.g.user.data
    last_data = None
    displayed_tab = flask.request.args.get('tab', 'profile')

    def set_data():
        profile_form.name.data = data.get('name', '')
        profile_form.name.description['placeholder'] = flask.g.user.wmbid
        profile_form.description.data = data.get('description', '')
        profile_form.gravatar.data = data.get('gravatar', '')
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
            if profile_form.name.data and not profile_form.name.data.isspace():
                data['name'] = profile_form.name.data
            else:
                data.pop('name', None)

            if profile_form.description.data and not profile_form.description.data.isspace():
                data['description'] = profile_form.description.data
            else:
                data.pop('description', None)

            if profile_form.gravatar.data and not profile_form.gravatar.data.isspace():
                data['gravatar'] = profile_form.gravatar.data
            else:
                data.pop('gravatar', None)

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
