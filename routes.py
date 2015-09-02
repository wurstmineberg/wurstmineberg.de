from flask import Blueprint, render_template, abort
from jinja2 import TemplateNotFound
from util import templated

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
