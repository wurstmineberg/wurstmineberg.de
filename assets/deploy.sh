#!/bin/sh

set -e

if [[ x"$(hostname -f)" == x'gharch.wurstmineberg.de' ]]; then
    # deploy wurstmineberg.de
    echo 'deploying wurstmineberg.de'
    rustup update stable
    env -C /opt/git cargo sweep -ir
    cd /opt/git/github.com/wurstmineberg/wurstmineberg.de/main
    sudo -u wurstmineberg git --git-dir=/opt/git/github.com/wurstmineberg/wurstmineberg.de/main/.git pull
    cargo build --release
    sudo systemctl stop wurstmineberg-web
    sudo chown wurstmineberg:wurstmineberg target/release/wurstmineberg-web
    sudo mv target/release/wurstmineberg-web /opt/wurstmineberg/bin/wurstmineberg-web
    sudo systemctl start wurstmineberg-web
    # reload caddy (since caddy config is tracked by git) and uWSGI
    sudo systemctl daemon-reload
    sudo systemctl reload caddy nginx uwsgi
else
    git push
    ssh wurstmineberg.de /opt/git/github.com/wurstmineberg/wurstmineberg.de/main/assets/deploy.sh
fi
