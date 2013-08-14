wurstmineberg-web
=================

wurstmannberg.de website

Trying to make collaborative website updates a little less funky.
Wurstmannberg.de website version is meant to be updated via pulls from here, with as little changes to the deployed website directly.  

Beware: On wurstmannberg.de, the proper remote is named "github" (don't ask, it's clusterfucky), so after you push changes to github, you have to

    cd ~/wmb.de/httpdocs && git pull github master


Please don't make this mess any worse.

## Bootstrap 3 version

Temporary BS3 files are placed in /bs3, among those are old versions of the site pages with some changed class names.
Turns out migrating from bs2 to bs3 is kind of sucky, so we probably need some branching technology to get that stuff figured out, which also means we have to get the bs3 files to merge content stuff from the master branch while keeping the updated class name stuff.  

I don't know yet how to handle that, so… Yeah. Feel free to help out.

## Credits

- CSS/JS is [Bootstrap](http://getbootstrap.com/) (v2)
- Basic index page layout was arranged via [Jetstrap](https://jetstrap.com/)