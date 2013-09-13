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

Temporary BS3 files are placed in /bs3, among those are old versions of the site pages with some changed class names.
Turns out migrating from bs2 to bs3 is kind of sucky, so we probably need some branching technology to get that stuff figured out, which also means we have to get the bs3 files to merge content stuff from the master branch while keeping the updated class name stuff.  

I don't know yet how to handle that, so… Yeah. Feel free to help out.

## Credits

- CSS/JS is [Bootstrap](http://getbootstrap.com/) (v2)
- Basic index page layout was arranged via [Jetstrap](https://jetstrap.com/)