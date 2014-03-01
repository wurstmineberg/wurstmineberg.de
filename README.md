wurstmineberg-web
=================

wurstmineberg.de website

Trying to make collaborative website updates a little less funky.
wurstmineberg.de is meant to be updated via pulls from here, with as little changes to the deployed website directly as possible.  

General management
------------------  
Like most of our repositories, this is also connected to our autodeploy setup, so pushed commits should go live on the website very soon.  

For bigger changes, see the dev branch of this repository, which is set up to be displayed on the [dev version of our site](http://dev.wurstmineberg.de/), which is handy for testing out experimental features.


Things in this repo
-------------------

- The actual website files (html, mostly)
- The config file for the overviewer (contains base markers and other POIs)
- Avatars of all users (used for /people.html, serverstatus and the overviewer base markers, so symlinks should be in place to make sure that works)

Things that will be in this repo
--------------------------------

If our overviewer config expands and possibly includes some more customization, we might get that in a separate repo, just to keep things a little better organized. After all, that's kind of the point.

Things that do not belong in this repo
--------------------------------------

- The overview files (tile images), because that would be just silly


Credits
-------

- CSS/JS is [Bootstrap 3](http://getbootstrap.com/)
