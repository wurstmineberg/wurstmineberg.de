from wurstmineberg_web import app

from flask_wtf import Form
from wtforms import StringField, TextAreaField
from wtforms.validators import DataRequired, ValidationError
import bleach


def html_whitelist_filter(data):
    tags = ['a', 'em', 's', 'span']
    attributes = {
        'span': lambda name, value: name == 'class' and value == 'muted',
        'a':    lambda name, value: name == 'href'
    }
    styles = ['']

    return bleach.clean(data, tags=tags, attributes=attributes, styles=styles, strip=True)

class MyForm(Form):
    name = StringField('Name', validators=[DataRequired()])
    description = TextAreaField('Description', filters=[html_whitelist_filter])
