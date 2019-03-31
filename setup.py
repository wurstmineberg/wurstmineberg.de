#!/usr/bin/env python

import setuptools

setuptools.setup(
    name='wurstmineberg.de',
    description='wurstmineberg.de website',
    author='Wurstmineberg',
    author_email='mail@wurstmineberg.de',
    packages=['wurstmineberg_web'],
    package_data={'wurstmineberg_web': ['templates/*.html']},
    install_requires=[
        'Flask',
        'Flask-Bootstrap',
        'Flask-Dance',
        'Flask-WTF',
        'Jinja2',
        'SQLAlchemy',
        'WTForms',
        'iso8601',
        'people',
        'pytz',
        'simplejson'
    ],
    dependency_links=[
        'git+https://github.com/wurstmineberg/people.git#egg=people'
    ]
)
