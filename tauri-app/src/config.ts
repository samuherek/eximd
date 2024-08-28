export type Path = string;

export type SrcFile = {
    ext: string,
    src: string,
    src_relative: string,
    stem: string
}

export type FileGroupImage = {
    key: string,
    image: SrcFile,
    config: SrcFile[],
    type: "Image",
}

export type FileGroupVideo = {
    key: string,
    video: SrcFile,
    config: SrcFile[],
    type: "Video",
}

export type FileGroupLiveImage = {
    key: string,
    image: SrcFile,
    video: SrcFile,
    config: SrcFile[],
    type: "LiveImage",
}

export type FileGroupUncertain = {
    key: string,
    primary: SrcFile[],
    config: SrcFile[],
    type: "Uncertain",
}

export type FileGroupUnsupported = {
    key: string,
    config: SrcFile[],
    type: "Unsupported",
}

export type FileGroupType = FileGroupImage | FileGroupVideo | FileGroupLiveImage | FileGroupUnsupported | FileGroupUncertain;
export type FileGroupToDisplay = FileGroupImage | FileGroupVideo | FileGroupLiveImage;
