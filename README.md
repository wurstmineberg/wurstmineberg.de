This repository contains the website **[wurstmineberg.de](https://wurstmineberg.de/)**, as well as the Discord bot `wurstminebot`.

# Installing

1. We use the [gitdir](https://github.com/fenhl/gitdir) directory structure. That means the website should be deployed to `/opt/git/github.com/wurstmineberg/wurstmineberg.de/main` and [the assets](https://github.com/wurstmineberg/assets.wurstmineberg.de) to `/opt/git/github.com/wurstmineberg/assets.wurstmineberg.de/main`.
2. The website consists of a portion written in Rust, which should be run using the systemd service file provided in `assets/wurstmineberg-web.service`, and a portion written in Python, which should be run in [uWSGI](https://uwsgi-docs.readthedocs.io/en/latest/). Both may be run behind [NGINX](https://nginx.com/) by creating symlinks to the `.nginx` files in `/etc/nginx/sites-available` and to the `.ini` files in `/etc/uwsgi/apps-available`, then creating symlinks to *those* in the respective `-enabled` directories.
3. The website also needs a [PostgreSQL](https://postgresql.org/) database named `wurstmineberg`.
4. For the remaining Python dependencies, each import is annotated with where you can find the package so `ImportError`s can be fixed directly. We also have a `setup.py` which may or may not work, sorry.

# General management

~~Like most of our repositories, this is also connected to [our autodeploy setup](https://github.com/fenhl/gitdir-autodeploy), so pushed commits should go live on the website very soon.~~ This is currently broken, so changes must be deployed using `gitdir deploy github.com wurstmineberg/wurstmineberg.de && sudo systemctl reload uwsgi`.

# Credits

* CSS/JS is [Bootstrap 3](http://getbootstrap.com/)
