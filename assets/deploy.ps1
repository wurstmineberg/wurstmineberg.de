git push
if (-not $?)
{
    throw 'Native Failure'
}

# copy the tree to the WSL file system to improve compile times
wsl rsync --delete -av /mnt/c/Users/fenhl/git/github.com/wurstmineberg/wurstmineberg.de/stage/ /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de/ --exclude target
if (-not $?)
{
    throw 'Native Failure'
}

wsl env -C /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de cargo build --release --target=x86_64-unknown-linux-musl
if (-not $?)
{
    throw 'Native Failure'
}

wsl cp /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de/target/x86_64-unknown-linux-musl/release/wurstmineberg-web /mnt/c/Users/fenhl/git/github.com/wurstmineberg/wurstmineberg.de/stage/target/wsl/release/wurstmineberg-web
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg@wurstmineberg.de env -C /opt/git/github.com/wurstmineberg/wurstmineberg.de/main git pull
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de sudo systemctl stop wurstmineberg-web
if (-not $?)
{
    throw 'Native Failure'
}

scp .\target\wsl\release\wurstmineberg-web wurstmineberg@wurstmineberg.de:/opt/wurstmineberg/bin/wurstmineberg-web
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de sudo systemctl daemon-reload
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de sudo systemctl reload nginx
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de sudo systemctl reload uwsgi
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de sudo systemctl start wurstmineberg-web
if (-not $?)
{
    throw 'Native Failure'
}
