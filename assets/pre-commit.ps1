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
wsl rsync --delete -av /mnt/c/Users/fenhl/git/github.com/wurstmineberg/wurstmineberg.de/stage/ /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de/ --exclude target
if (-not $?)
{
    throw 'Native Failure'
}

wsl env -C /home/fenhl/wslgit/github.com/wurstmineberg/wurstmineberg.de cargo check
if (-not $?)
{
    throw 'Native Failure'
}
