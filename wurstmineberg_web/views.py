from flask import Blueprint, render_template, abort, redirect, g
from jinja2 import TemplateNotFound
from .util import templated
from .models import Person

from flask.ext.login import login_required, logout_user

from wurstmineberg_web import app

page = Blueprint('page', __name__, template_folder='templates/page')

@page.route('/')
@templated('index.html')
def index():
    return None

@page.route('/about')
@templated()
def about():
    return None

@page.route('/stats')
@templated()
def stats():
    return None

@page.route('/people')
@templated()
def people():
    return None

@page.route('/people/<person>')
@templated('people_detail.html')
def people_detail(person):
    return {'person': person}

@page.route('/login')
def login():
    if g.user and g.user.is_authenticated():
        raise KeyError(g.user)
        return redirect('/profile')
    else:
        return render_template('login.html')

@login_required
@page.route('/login_done')
@templated()
def login_done():
    return None

@page.route('/logout')
def logout():
    """Logout view"""
    logout_user()
    return redirect('/')
