<div align="center">

# ExiMd

![eximd](https://github.com/user-attachments/assets/cca8b788-4d2f-4e43-84e2-4096642bfbb6)

Eeasily rename your media into a timestamp file names.



https://github.com/user-attachments/assets/fc3514c6-7ba5-4187-8714-026f4935b7e4



</div>

---

## Overview

This is a simple application that would rename your media files from the usual camera file names like `IMG_87878.JPGz` to a more managable name similar to how dropbox backups the photos. 
The new name will be of the format `YYYY-MM-DD_HH.MM.SS.ext`.

The purpose of this is to easily manage your files. As it's very often that if you move the file to a new storage or make some edits,
the exif timestamp can be overwritten. But mainly, having the standard file names mean you can have duplicates and it's harded to manage. 

This app also taks into account extra files like the edit files from apple or adobe (`xpm`, `aae`) and makes sure that it will 
have the same "name" as the original media that actually has the timestamp. This way, your edits will still be stored alongside of the files. 
The same should be the case for the live photos as the video and the photo will end up having the same file name.

This has been a big pain for me personally, and thus this little app. 

## Usage

Do a dry run without a rename to see what the output is going to be:

`eximd rename "path/to/dir_or_file"`

If you are sure that you are ok with the next file names, you can commit the changes with:

`eximd rename --exec "path/to/dir_or_file"`

## TODO:

- Make a UI for people who don't want to use the cli app.
- Expand the app to "tag" the photos with GPS coordinates
- Make a visualization on the map of where the files are localted and how they moved acorss time
