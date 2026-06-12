#!/bin/sh

set -e

if [[ x"$(hostname -f)" == x'gharch.wurstmineberg.de' ]]; then
    # deploy wurstmineberg.de
    echo 'deploying wurstmineberg.de'
    cd /opt/git/github.com/wurstmineberg/wurstmineberg.de/main
    git --git-dir=/opt/git/github.com/wurstmineberg/wurstmineberg.de/main/.git pull
    sudo systemctl stop wurstmineberg-web
    mv /opt/wurstmineberg/bin/wurstmineberg-web-next /opt/wurstmineberg/bin/wurstmineberg-web
    sudo systemctl start wurstmineberg-web
    # reload caddy (since caddy config is tracked by git) and uWSGI
    sudo systemctl daemon-reload
    sudo systemctl reload caddy nginx uwsgi
else
    git push
    cargo build --release
    scp target/release/wurstmineberg-web wurstmineberg.de:/opt/wurstmineberg/bin/wurstmineberg-web-next
    ssh wurstmineberg.de /opt/git/github.com/wurstmineberg/wurstmineberg.de/main/assets/deploy.sh
fi
