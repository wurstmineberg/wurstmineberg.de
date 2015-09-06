from flask import render_template, abort, redirect, g
from jinja2 import TemplateNotFound
from .util import templated
from .models import Person

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

@app.route('/preferences')
@login_required
@templated()
def preferences():
    return None

@app.route('/logout')
@login_required
def logout():
    """Logout view"""
    logout_user()
    return redirect('/')

