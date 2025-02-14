#!/bin/zsh

set -e

if [[ x"$(hostname -f)" == x'gharch.wurstmineberg.de' ]]; then
    # deploy wurstmineberg.de
    echo 'deploying wurstmineberg.de'
    cd /opt/git/github.com/wurstmineberg/wurstmineberg.de/main
    git --git-dir=/opt/git/github.com/wurstmineberg/wurstmineberg.de/main/.git pull
    # restart nginx (since nginx config is tracked by git) and uWSGI
    sudo systemctl daemon-reload
    sudo systemctl reload nginx
    sudo systemctl reload uwsgi
else
    git push
    cargo build --release --target=x86_64-unknown-linux-musl
    scp target/x86_64-unknown-linux-musl/release/wurstmineberg-web wurstmineberg.de:/opt/wurstmineberg/bin/wurstmineberg-web
    ssh wurstmineberg.de /opt/git/github.com/wurstmineberg/wurstmineberg.de/main/assets/deploy.sh
fi
