<div align="center">

# ExiMd

> Simple an effective sanitizer of your media file names even when it's a live photo or it has extra edit files with it. 


https://github.com/user-attachments/assets/3b496e4f-d95a-4bc4-bc98-0e2f7534afd1


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

