git push
if (-not $?)
{
    throw 'Native Failure'
}

ssh wurstmineberg.de /opt/git/github.com/wurstmineberg/wurstmineberg.de/main/assets/deploy.sh
if (-not $?)
{
    throw 'Native Failure'
}
