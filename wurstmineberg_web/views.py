from flask import render_template, abort, redirect, g, request, flash
from jinja2 import TemplateNotFound
from .util import templated
from .models import Person
from .forms import *

from flask.ext.login import login_required, logout_user

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
    person = Person.get_person(wmbid)
    if not person:
        return abort(404)
    return {'wmbid': wmbid, 'person': person}

@app.route('/profile')
@login_required
def get_profile():
    return redirect('/people/{}'.format(g.user.wmbid))

@app.route('/login')
def login():
    if g.user and g.user.is_authenticated():
        return redirect('/profile')
    else:
        return render_template('login.html')

@app.route('/preferences', methods=('GET', 'POST'))
@login_required
@templated()
def preferences():
    form = MyForm()
    data = g.user.person.data

    def set_data():
        form.name.data = data.get('name', '')
        if not form.name.data:
            form.name.description['placeholder'] = g.user.wmbid
        form.description.data = data.get('description', '')
        form.gravatar.data = data.get('gravatar', '')
        form.mojira.data = data.get('mojira', '')
        form.twitter.data = data.get('twitter', {}).get('username', '')
        form.website.data = data.get('website', None)
        form.favcolor.color_dict = data.get('favColor', None)

    if request.method == 'GET':
        set_data()
    if form.validate_on_submit():
        if form.name.data and not form.name.data.isspace():
            data['name'] = form.name.data
        else:
            data.pop('name', None)

        if form.description.data and not form.description.data.isspace():
            data['description'] = form.description.data
        else:
            data.pop('description', None)

        if form.gravatar.data and not form.gravatar.data.isspace():
            data['gravatar'] = form.gravatar.data
        else:
            data.pop('gravatar', None)

        if form.mojira.data and not form.mojira.data.isspace():
            data['mojira'] = form.mojira.data
        else:
            data.pop('mojira', None)

        if form.twitter.data and not form.twitter.data.isspace():
            data.setdefault('twitter', {})['username'] = form.twitter.data
        else:
            data.pop('twitter', None)

        if form.website.data and not form.website.data.isspace():
            data['website'] = form.website.data
        else:
            data.pop('website', None)

        color_dict = form.favcolor.color_dict
        if color_dict:
            data['favColor'] = color_dict
        else:
            data.pop('favColor', None)

        g.user.person.commit_data()
        set_data()
        flash('Successfully saved data')

    return {'form': form}

@app.route('/logout')
@login_required
def logout():
    """Logout view"""
    logout_user()
    return redirect('/')

