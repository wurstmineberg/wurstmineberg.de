wurstmineberg-web
=================

wurstmineberg.de website

Trying to make collaborative website updates a little less funky.
wurstmineberg.de is meant to be updated via pulls from here, with as little changes to the deployed website directly as possible.  

### Update for the new server:
Since stuff is organized nicely now, we can do

    cd /opt/hub/wurstmineberg/wurstmineberg-web; git pull origin master
    
And everything should be fine and dandy.

## Things in this repo
- The actual website files (html, bootstrap stuff)
- The config file for the overviewer (contains base markers and other POIs)
- Avatars of all users (used for /people.html, serverstatus and the overviewer base markers, so symlinks should be in place to make sure that works)

## Things that will be in this repo
TODO

## Things that do not belong in this repo
- The overview files, because that would be just silly
    

## Bootstrap 3 version

Maybe we do, maybe we don't.

## Credits

- CSS/JS is [Bootstrap](http://getbootstrap.com/) (v2)
- Basic index page layout was arranged via [Jetstrap](https://jetstrap.com/)