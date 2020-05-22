import flask_pagedown.fields # PyPI: Flask-PageDown
import flask_wtf # PyPI: Flask-WTF
import jinja2 # PyPI: jinja2
import pytz # PyPI: pytz
import wtforms # PyPI: WTForms
import wtforms.validators # PyPI: WTForms
import wtforms.widgets # PyPI: WTForms

def twitter_username_filter(username):
    if username and len(username) >= 1 and username[0] == u'@':
        return username[1:]
    return username

_email_validator = wtforms.validators.Email()

class EmptyOrValidatorValidator():
    def __init__(self, validator):
        self.validator = validator

    def __call__(self, form, field):
        if field.data and not field.data.isspace():
            return self.validator(form, field)
        return True

class ColorWidget(wtforms.widgets.Input):
    def __init__(input_type=None):
        return super().__init__(input_type=input_type)

    def __call__(self, field, **kwargs):
        ret = '<span class="form-colorpicker input-group" data-format="hex">'
        ret += super().__call__(field, **kwargs)
        ret += '<span class="input-group-addon"><i></i></span>'
        ret += '</span>'
        return jinja2.Markup(ret)


import binascii
class ColorField(wtforms.StringField):

    def __init__(self, *args, validators=[], **kwargs):
        validators.append(EmptyOrValidatorValidator(wtforms.validators.Regexp('#([0-9a-f]{6})')))
        super().__init__(*args, validators=validators, **kwargs)
        self.widget = ColorWidget()
        self.data_default = {'red': None, 'green': None, 'blue': None}
        self.data = self.data_default

    @property
    def color_dict(self):
        if self.data and len(self.data) == 7:
            red = int(self.data[1:3], 16)
            green = int(self.data[3:5], 16)
            blue = int(self.data[5:7], 16)
            return {
                'red': red,
                'green': green,
                'blue': blue
            }
        else:
            return None

    @color_dict.setter
    def color_dict(self, value):
        if value:
            self.data = '#{:02x}{:02x}{:02x}'.format(value['red'], value['green'], value['blue'])

class MarkdownField(flask_pagedown.fields.PageDownField):
    def _value(self):
        import wurstmineberg_web.wiki

        if self.raw_data:
            return self.raw_data[0]
        elif self.data is not None:
            return wurstmineberg_web.wiki.mentions_to_tags(self.data)
        else:
            return ''

    def process_formdata(self, valuelist):
        import wurstmineberg_web.wiki

        if valuelist:
            self.data = wurstmineberg_web.wiki.tags_to_mentions(valuelist[0])

class ProfileForm(flask_wtf.FlaskForm):
    #TODO clarify that the name field doesn't work for Discord admins
    name = wtforms.StringField('Name', validators=[wtforms.validators.Regexp('^([^@#:]{2,32})$')], description={ #TODO better compliance with https://discord.com/developers/docs/resources/user
        'text': "The name that will be used when addressing you and referring to you. If you're in our Discord server, this will be kept in sync with your display name there."})
    description = MarkdownField('Description',
        description={
            'text': '1000 characters maximum.',
            'placeholder': 'A short text (up to 1000 characters) that describes you. May contain Markdown formatting.'},
        validators=[wtforms.validators.Length(max=1000)])
    mojira = wtforms.StringField('Mojira username',
        description={'text': 'Your username on the Mojira bug tracker'},
        validators=[wtforms.validators.Length(max=50)])
    twitter = wtforms.StringField('Twitter username',
        description={'text': 'Your Twitter @username'},
        filters=[twitter_username_filter],
        validators=[EmptyOrValidatorValidator(wtforms.validators.Regexp('[A-Za-z0-9_]+')),
                    wtforms.validators.Length(max=15)])
    website = wtforms.StringField('Website',
        description={'text': 'The URL of your website',
            'placeholder': 'http://www.example.com'},
        validators=[EmptyOrValidatorValidator(wtforms.validators.URL()),
                    wtforms.validators.Length(max=2000)])
    favcolor = ColorField('Favorite Color',
        description={'text': 'Your favorite color, used for statistics',
            'placeholder': 'Enter a hex RGB color like #000000 or use the color picker on the right'},
        widget=ColorWidget())
    submit_profile_form = wtforms.SubmitField('Save')

def SettingsForm():
    options = {
        'allow_online_notifications': {
            'name': 'Allow others to receive online notifications for you',
            'description': 'This website will soon™ have a feature where members can ask to receive notifications when players join/leave the main world. If you disable this setting, no one will receive these notifications when you join/leave.',
            'default': True
        },
        #'activity_tweets': {
        #    'name': 'Activity Tweets',
        #    'description': 'When this option is off, the bot will refrain from @mentioning you in\
        #        achievement and death tweets (this feature is not yet implemented).',
        #    'default': True
        #},
        #'inactivity_tweets': {
        #    'description': 'When this option is on, the bot will send you a tweet after a random\
        #        time (between 1 and 6 months) of inactivity (this feature is not yet implemented,\
        #        see here for the feature request) and on your whitelisting anniversary (not yet\
        #        implemented either, see here for the feature request). When it\'s off, it will\
        #        still tweet about your anniversary, but without @mentioning you.'
        #},
        'public_info': {
            'name': 'User data is public',
            'description': 'When this option is off, only server members logged in on the website can view your profile page and statistics. Note that your data is still publicly accessible via the API.',
            'default': True
        },
        'show_inventory': {
            'name': 'Show inventory',
            'description': 'Whether or not your profile page should show your inventory and Ender chest content.',
            'default': False
        }
    }

    class Form(flask_wtf.FlaskForm):
        pass

    for name, option in options.items():
        value = False
        if 'default' in option:
            value = option['default']
        field = wtforms.BooleanField(option['name'],
            default=value,
            description={'text': option['description']})
        setattr(Form, name, field)
    common_timezones = ['Etc/UTC', 'Europe/Berlin', 'Europe/Vienna']
    timezones = common_timezones + [timezone for timezone in pytz.all_timezones if timezone not in common_timezones]
    Form.timezone = wtforms.SelectField('Time zone', default='Etc/UTC', choices=[(timezone, timezone) for timezone in timezones], coerce=pytz.timezone)
    Form.submit_settings_form = wtforms.SubmitField('Save')

    return Form()
