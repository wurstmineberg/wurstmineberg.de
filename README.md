wurstmineberg-web
=================

wurstmineberg.de website

Trying to make collaborative website updates a little less funky.
wurstmineberg.de is meant to be updated via pulls from here, with as little changes to the deployed website directly as possible.  

### Update for the new server:
Since stuff is organized nicely now, we can do

    cd /opt/hub/wurstmineberg/wurstmineberg-web; git pull origin master
    
And everything should be fine and dandy.  
You can also use this handy alias:

    alias pullweb='cd /opt/hub/wurstmineberg/wurstmineberg-web/; git pull origin master; cd -'

Things in this repo
-------------------

- The actual website files (html, mostly)
- The config file for the overviewer (contains base markers and other POIs)
- Avatars of all users (used for /people.html, serverstatus and the overviewer base markers, so symlinks should be in place to make sure that works)

Things that will be in this repo
--------------------------------

If our overviewer config expands and possibly included some more cutsomization, we might get that in a separat repo, just to keep things a little better organized. After all, that's kind of the point.

Things that do not belong in this repo
--------------------------------------

- The overview files (tile images), because that would be just silly


Credits
-------

- CSS/JS is [Bootstrap 3](http://getbootstrap.com/)
