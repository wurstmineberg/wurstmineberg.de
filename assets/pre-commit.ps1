#!/usr/bin/env pwsh

cargo check
if (-not $?)
{
    throw 'Native Failure'
}

cargo sqlx prepare --check
if (-not $?)
{
    throw 'Native Failure'
}

# copy the tree to the WSL file system to improve compile times
wsl -d ubuntu-m2 rsync --mkpath --delete -av /mnt/c/Users/fenhl/git/github.com/wurstmineberg/wurstmineberg.de/stage/ /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de/ --exclude target
if (-not $?)
{
    throw 'Native Failure'
}

wsl -d ubuntu-m2 env -C /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de /home/fenhl/.cargo/bin/cargo check
if (-not $?)
{
    throw 'Native Failure'
}
