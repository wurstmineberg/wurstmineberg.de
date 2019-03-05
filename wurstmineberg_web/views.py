from flask import render_template, abort, redirect, g, request, flash
from jinja2 import TemplateNotFound
from .util import templated
from .models import Person
import wurstmineberg_web.forms as forms
import wtforms

from flask_login import login_required, logout_user

from wurstmineberg_web import app


@app.route('/')
@templated('index.html')
def index():
    return None

@app.route('/about')
@templated()
def about():
    return None

@app.route('/stats')
@templated()
def stats():
    return None

@app.route('/people/')
@templated()
def people():
    people = Person.get_people_ordered_by_status()
    for key in ['founding', 'later', 'former', 'guest', 'invited', 'vetoed']:
        people[key] = people.get(key, [])

    people['guest'].extend(people['invited'])
    people['former'].extend(people['vetoed'])
    return {'people': people}

@app.route('/people/<wmbid>')
@templated('profile.html')
def profile(wmbid):
    person = Person.from_wmbid(wmbid)
    if not person:
        return abort(404)
    return {'wmbid': wmbid, 'person': person}

@app.route('/profile')
@login_required
def get_profile():
    return redirect('/people/{}'.format(g.user.wmbid))

@app.route('/preferences', methods=('GET', 'POST'))
@login_required
@templated()
def preferences():
    profile_form = forms.ProfileForm()
    settings_form = forms.SettingsForm()
    data = g.user.person.data
    last_data = None
    displayed_tab = request.args.get('tab', 'profile')

    def set_data():
        profile_form.name.data = data.get('name', '')
        profile_form.name.description['placeholder'] = g.user.wmbid
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

    if request.method == 'GET':
        set_data()
    elif request.method == 'POST' and 'save' in request.form:
        submitted_form = request.form['save']
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

            g.user.person.commit_data()
            set_data()
            flash('Successfully saved profile')

        if submitted_form == 'save-settings' and settings_form.validate():
            options = data.setdefault('options', {})
            for field in settings_form:
                if not isinstance(field, wtforms.HiddenField):
                    options[field.id] = field.data
            data['options'] = options
            g.user.person.commit_data()
            set_data()
            flash('Successfully saved settings')


    return {'profile_form': profile_form, 'settings_form': settings_form, 'displayed_tab': displayed_tab}
