<div align="center">

# ExiMd

**Simple an effective media files sanitizer that respect related edit and config files.**


https://github.com/user-attachments/assets/1ffccf46-0295-4959-bef9-ddf28c93b62c


</div>

---

**!!!! This is still WIP but soon to be released !!!!**


## Why this exists and what it does

**My problem is summed up as. I would like to:**

- rename my pictures the way "dropbox" backup did it. Format: `YY-MM-DD HH.MM.SS`
- do this even for older photos that have been moved around on hard disks a bit and have different "creation dates"
- the tool to rename **related** files too and not forget about them:
    - Apple's live photo,
    - Adobe lightroom metadata file for raw images
    - Apple's non-destructive iphone edit files in "aae"
    - ....
  
### What this tool does

- It scans a directory and groups files based on their name similiarity. (preserving relationship with edit files)
- Then peaks in to the exif metadata of the primary file and tries to get the "creation date"
- Shows you the old name and new name and whether the file has related files.
- Renames the files you selected you wanted to be renamed

**Note**

If it's difficutl to determin if the media is:

- Photo and some metadata files
- Photo and a video file and some metadata files
- Video and some metadata files

Then it shows you the groupings and won't do the renaming. This is based on the usecase I've found so far. In case you have other use cases, please feel free to open an issue and describe the case. I might end up adding it if it make sense and it helps you with your files. 


## It is alos a CLI app

If you fency a cli app instead. You can download it with **Homebrew**

- tap
- 

