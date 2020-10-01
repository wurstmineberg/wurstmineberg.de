This repository contains the website **[wurstmineberg.de](https://wurstmineberg.de/)**.

wurstmineberg.de is meant to be updated via pulls from here, with as little changes to the deployed website directly as possible.

# Installing

1. We use the [gitdir](https://github.com) directory structure. That means the website should be deployed to `/opt/git/github.com/wurstmineberg/wurstmineberg.de/master` and [the assets](https://github.com/wurstmineberg/assets.wurstmineberg.de) to `/opt/git/github.com/wurstmineberg/assets.wurstmineberg.de/master`.
2. The website is designed to run on [uWSGI](https://uwsgi-docs.readthedocs.io/en/latest/) behind [NGINX](https://nginx.com/). Create symlinks to the `.nginx` files in `/etc/nginx/sites-available` and to the `.ini` files in `/etc/uwsgi/apps-available`, then create symlinks to *those* in the respective `-enabled` directories.
3. The website also needs a [PostgreSQL](https://postgresql.org/) database named `wurstmineberg`, as well as a running [wurstminebot](https://github.com/wurstmineberg/wurstminebot-discord) to keep user data from Discord up to date.
4. For the Python dependencies, each import is annotated with where you can find the package so `ImportError`s can be fixed directly. We also have a `setup.py` which may or may not work, sorry.

# General management

~~Like most of our repositories, this is also connected to [our autodeploy setup](https://github.com/fenhl/gitdir-autodeploy), so pushed commits should go live on the website very soon.~~ This is currently broken, so changes must be deployed using `gitdir deploy github.com wurstmineberg/wurstmineberg.de && sudo systemctl reload uwsgi`.

# Credits

* CSS/JS is [Bootstrap 3](http://getbootstrap.com/)
