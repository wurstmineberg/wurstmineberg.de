#!/usr/bin/env python3
"""
Wurstmineberg website
"""

from wurstmineberg_web import create_app

# uwsgi starts the application differently
production = __name__ != '__main__'
debug = not production

application = create_app(production)

if __name__ == '__main__':
	application.run(debug=debug)

