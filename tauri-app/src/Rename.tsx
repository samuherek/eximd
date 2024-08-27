import { ActorRefFrom, assign, enqueueActions, fromCallback, fromPromise, sendParent, sendTo, setup } from "xstate";
import { useSelector } from "@xstate/react";
import { raiseErrorToUI } from './utils';
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { toast } from "react-toastify";

type Props = {
    actorRef: ActorRefFrom<typeof renameMachine>
}

type SrcFile = {
    ext: string,
    src: string,
    src_relative: string,
    stem: string
}

type FileGroupImage = {
    key: string,
    image: SrcFile,
    config: SrcFile[],
    type: "Image",
}

type FileGroupVideo = {
    key: string,
    video: SrcFile,
    config: SrcFile[],
    type: "Video",
}

type FileGroupLiveImage = {
    key: string,
    image: SrcFile,
    video: SrcFile,
    config: SrcFile[],
    type: "LiveImage",
}

type FileGroupUncertain = {
    key: string,
    primary: SrcFile[],
    config: SrcFile[],
    type: "Uncertain",
}

type FileGroupUnsupported = {
    key: string,
    config: SrcFile[],
    type: "Unsupported",
}

type FileGroupType = FileGroupImage | FileGroupVideo | FileGroupLiveImage | FileGroupUnsupported | FileGroupUncertain;
type FileGroupToDisplay = FileGroupImage | FileGroupVideo | FileGroupLiveImage;

const tauriCollectCommand = fromPromise(async () => {
    const res = await invoke<{ files: FileGroupType[], file_count: number }>("collect_rename_files_cmd");
    const [items, uncertain, unsupported] = res.files.reduce((prev, next) => {
        if (next.type === "Unsupported") {
            prev[2].push(next);
        } else if (next.type === "Uncertain") {
            prev[1].push(next);
        } else {
            prev[0].push(next);
        }
        return prev;
    }, [[] as FileGroupToDisplay[], [] as FileGroupUncertain[], [] as FileGroupUnsupported[]]);

    return {
        items,
        uncertain,
        unsupported,
        file_count: res.file_count
    }
});

const tauriExifCollectCommand = fromPromise(async () => {
    const res = await invoke<null>("start_exif_collection_cmd");
    return res;
});

const tauriCommitRenameGroupsCommand = fromPromise(async ({ input }) => {
    const res = await invoke<null>("commit_rename_groups_cmd", { payload: { items: input } });
    return res;
});

type ExifFileDataEvent = {
    key: string,
    src: string,
    src_next: string,
    file_name_next: string,
}

const tauriExifDataListener = fromCallback(({ sendBack }) => {
    const unlisten = listen<{
        key: string,
        src: string,
        src_next: string,
        file_name_next: string
    }>("EXIF_FILE_DATA", (data) => {
        sendBack({
            type: "EXIF_FILE_DATA", payload: {
                key: data.payload.key,
                src: data.payload.src,
                src_next: data.payload.src_next,
                file_name_next: data.payload.file_name_next
            } as ExifFileDataEvent
        })
    })
    const doneUnlisten = listen("EXIF_COLLECTION_DONE", () => {
        sendBack({ type: "EXIF_COLLECTION_DONE" });
    });
    return () => {
        unlisten.then(fn => fn())
        doneUnlisten.then(fn => fn())
    }
});

const tauriCommitRenameDoneListener = fromCallback(({ sendBack }) => {
    const unlisten = listen<string>("RENAME_COMMIT_SUCCESS_MSG", (data) => {
        console.log("commit done data ", data);
        sendBack({ type: "RENAME_COMMIT_SUCCESS", payload: data.payload })
    });
    const doneUnlisten = listen("RENAME_COMMIT_DONE_MSG", () => {
        sendBack({ type: "RENAME_COMMIT_DONE" });
    });
    return () => {
        unlisten.then(fn => fn());
        doneUnlisten.then(fn => fn());
    }
});


const unsupportedItemMachine = setup({
    types: {} as {
        context: {
            file: FileGroupUnsupported
        },
        input: FileGroupUnsupported
    },
}).createMachine({
    context: ({ input }) => ({
        file: input,
    })
});

const uncertainItemMachine = setup({
    types: {} as {
        context: {
            file: FileGroupUncertain
        },
        input: FileGroupUncertain
    },
}).createMachine({
    context: ({ input }) => ({
        file: input,
    })
});

const supportedItemMachine = setup({
    types: {} as {
        context: {
            file: FileGroupToDisplay,
            selected: boolean,
            src_next: string | null,
            file_name_next: string | null
            isCommitted: boolean,
        },
        input: FileGroupToDisplay,
        events: { type: "DESELECT_ITEM" }
        | { type: "SELECT_ITEM" }
        | { type: "SELECT_ITEM_FROM_PARENT" }
        | { type: "DESELECT_ITEM_FROM_PARENT" }
        | { type: "SET_NEXT_STEM", payload: ExifFileDataEvent }
        | { type: "RENAME_COMMIT_SUCCESS_FROM_PARENT" }
    },
    guards: {
        isSelected: ({ context }) => context.selected === true,
        isDeselected: ({ context }) => context.selected === false,
    }
}).createMachine({
    initial: "exifing",
    context: ({ input }) => ({
        file: input,
        selected: true,
        src_next: null,
        file_name_next: null,
        isCommitted: false
    }),
    on: {
        DESELECT_ITEM: {
            guard: 'isSelected',
            actions: [
                assign({ selected: () => false }),
                sendParent({ type: "DESELECT_ITEM" })
            ]
        },
        SELECT_ITEM: {
            guard: 'isDeselected',
            actions: [
                assign({ selected: () => true }),
                sendParent({ type: "SELECT_ITEM" })
            ]
        },
        DESELECT_ITEM_FROM_PARENT: {
            guard: 'isSelected',
            actions: [
                assign({ selected: () => false }),
            ]
        },
        SELECT_ITEM_FROM_PARENT: {
            guard: 'isDeselected',
            actions: [
                assign({ selected: () => true }),
            ]
        },
    },
    states: {
        exifing: {
            on: {
                SET_NEXT_STEM: {
                    target: 'ready',
                    actions: assign({
                        src_next: ({ event }) => event.payload.src_next,
                        file_name_next: ({ event }) => event.payload.file_name_next,
                    })
                }
            }
        },
        ready: {
            on: {
                RENAME_COMMIT_SUCCESS_FROM_PARENT: {
                    target: "done",
                    actions: assign({
                        isCommitted: () => true
                    })
                }
            }
        },
        done: {
            on: {
                DESELECT_ITEM: {},
                SELECT_ITEM: {},
                DESELECT_ITEM_FROM_PARENT: {},
                SELECT_ITEM_FROM_PARENT: {},
            }
        }
    }
});

const renameMachine = setup({
    types: {} as {
        context: {
            items: ActorRefFrom<typeof supportedItemMachine>[],
            uncertain: ActorRefFrom<typeof uncertainItemMachine>[];
            unsupported: ActorRefFrom<typeof unsupportedItemMachine>[],
            source: string,
            file_count: number | null,
            selected_all: boolean,
            selected_count: number,
        },
        events: { type: "TOGGLE_SELECTION_ALL" }
        | { type: "EXIF_FILE_DATA", payload: ExifFileDataEvent }
        | { type: "RESET_TO_INTRO" }
        | { type: "EXIF_COLLECTION_DONE" }
        | { type: "SELECT_ITEM" }
        | { type: "DESELECT_ITEM" }
        | { type: "SELECT_ALL" }
        | { type: "DESELECT_ALL" }
        | { type: "COMMIT_RENAME_GROUPS" }
        | { type: "RENAME_COMMIT_SUCCESS", payload: string }
        | { type: "RENAME_COMMIT_DONE" }
    },
    actors: {
        tauriCollectCommand,
        tauriExifCollectCommand,
        tauriExifDataListener,
        tauriCommitRenameGroupsCommand,
        tauriCommitRenameDoneListener
    },
    actions: {
        updateItemSelection: enqueueActions(({ context, enqueue }) => {
            const allCount = context.items.length;
            const count = context.items.filter((item) => {
                return item.getSnapshot().context.selected;
            }).length;
            enqueue.assign({
                selected_all: allCount === count,
                selected_count: count
            })
        })
    }
}).createMachine(
    {
        id: "rename-machine",
        initial: 'collecting',
        context: ({ input }: any) => ({
            items: [],
            uncertain: [],
            unsupported: [],
            source: input.source,
            file_count: null,
            selected_all: true,
            selected_count: 0,
        }),
        states: {
            collecting: {
                invoke: {
                    src: 'tauriCollectCommand',
                    onDone: {
                        target: 'exifing',
                        actions: assign({
                            // @ts-ignore
                            items: ({ event, spawn }) => {
                                return event.output.items.map(
                                    // @ts-ignore
                                    (item) => spawn(supportedItemMachine, {
                                        id: item.key,
                                        input: item
                                    })
                                )
                            },
                            // @ts-ignore
                            uncertain: ({ event, spawn }) => {
                                return event.output.uncertain.map(item => {
                                    // @ts-ignore
                                    return spawn(uncertainItemMachine, {
                                        id: item.key,
                                        input: item
                                    })
                                })
                            },
                            // @ts-ignore
                            unsupported: ({ event, spawn }) => {
                                return event.output.unsupported.map(item => {
                                    // @ts-ignore
                                    return spawn(unsupportedItemMachine, {
                                        id: item.key,
                                        input: item
                                    })
                                })
                            },
                            file_count: ({ event }) => event.output.file_count,
                            selected_count: ({ event }) => event.output.items.length
                        })
                    },
                    onError: {
                        actions: raiseErrorToUI
                    }
                },
                on: {
                    TOGGLE_SELECTION_ALL: {}
                }
            },
            exifing: {
                initial: "start",
                invoke: {
                    src: tauriExifDataListener,
                },
                states: {
                    start: {
                        invoke: {
                            src: tauriExifCollectCommand,
                            onDone: {
                                target: "loading",
                            },
                            onError: {
                                actions: raiseErrorToUI
                            }
                        },
                    },
                    loading: {
                        entry: () => { },
                        on: {
                            EXIF_FILE_DATA: {
                                actions: sendTo(
                                    // The "key" is the key of the item machine. If it changes,
                                    // make sure to change it as well.
                                    ({ event }) => event.payload.key,
                                    ({ event }) => ({ type: "SET_NEXT_STEM", payload: event.payload })
                                ),
                            },
                            EXIF_COLLECTION_DONE: '#rename-machine.ready'
                        }
                    },
                },
            },
            ready: {
                on: {
                    COMMIT_RENAME_GROUPS: 'committing',
                }
            },
            committing: {
                initial: "start",
                invoke: {
                    src: "tauriCommitRenameDoneListener",
                },
                states: {
                    start: {
                        invoke: {
                            src: 'tauriCommitRenameGroupsCommand',
                            onDone: "loading",
                            onError: {
                                actions: raiseErrorToUI,
                            },
                            input: ({ context }) => {

                                const toRename = context.items
                                    .filter(x => x.getSnapshot().context.selected)
                                    .map(x => x.getSnapshot().context.file.key)
                                console.log("keys to rename", toRename);
                                return toRename
                            }
                        },
                    },
                    loading: {
                        on: {
                            RENAME_COMMIT_SUCCESS: {
                                actions: [(data) => console.log("we are here", data),
                                sendTo(({ event }) => event.payload, { type: "RENAME_COMMIT_SUCCESS_FROM_PARENT" })
                                ]
                            },
                            RENAME_COMMIT_DONE: {
                                target: "#rename-machine.done"
                            }
                        }
                    },
                },
                on: {
                    TOGGLE_SELECTION_ALL: {},
                    RESET_TO_INTRO: {}
                }
            },
            done: {
                entry: () => toast("Done renaming", { type: "success" })
            }
        },
        on: {
            TOGGLE_SELECTION_ALL: {
                actions: enqueueActions(({ context, enqueue }) => {
                    const isEverySelected = context.items.every(x => {
                        const snap = x.getSnapshot();
                        return snap.context.selected;
                    });
                    if (isEverySelected) {
                        enqueue.raise({ type: "DESELECT_ALL" });
                    } else {
                        enqueue.raise({ type: "SELECT_ALL" });
                    }
                }),
            },
            SELECT_ITEM: {
                actions: "updateItemSelection"
            },
            DESELECT_ITEM: {
                actions: "updateItemSelection"
            },
            SELECT_ALL: {
                actions: [({ context }) => {
                    context.items.forEach(x => {
                        x.send({ type: "SELECT_ITEM_FROM_PARENT" });
                    })
                }, assign({
                    selected_all: () => true,
                    selected_count: ({ context }) => context.items.length
                })]
            },
            DESELECT_ALL: {
                actions: [({ context }) => {
                    context.items.forEach(x => {
                        x.send({ type: "DESELECT_ITEM_FROM_PARENT" });
                    })
                }, assign({
                    selected_all: () => false,
                    selected_count: () => 0
                })]
            },
            RESET_TO_INTRO: {
                actions: sendParent({ type: "RESET_TO_INTRO" })
            }
        }
    }
);

function FileGroupVideo({ item }: { item: SrcFile }) {
    return (
        <>
            <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512">
                <path className="fa-secondary" opacity=".4" d="M0 288c0 17.7 14.3 32 32 32l32 0 0 128c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-16 0-96 0-16c0-35.3-28.7-64-64-64l-32 0-224 0-64 0-32 0c-17.7 0-32 14.3-32 32z" />
                <path className="fa-primary" d="M128 64a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM352 256l-224 0C57.3 256 0 198.7 0 128S57.3 0 128 0c48.2 0 90.2 26.6 112 66C261.8 26.6 303.8 0 352 0c70.7 0 128 57.3 128 128s-57.3 128-128 128zm0-192a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM558.3 259.4c10.8 5.4 17.7 16.5 17.7 28.6l0 192c0 12.1-6.8 23.2-17.7 28.6s-23.8 4.3-33.5-3l-64-48L448 448l0-16 0-96 0-16 12.8-9.6 64-48c9.7-7.3 22.7-8.4 33.5-3z" />
            </svg>
            <span>{item.stem}.{item.ext}</span>
        </>
    )
}

function FileGroupLiveImage({ item }: { item: SrcFile }) {
    return (
        <>
            <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path opacity=".4" d="M0 96C0 60.7 28.7 32 64 32l384 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm64 48a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zm2.4 258.4c4 8.3 12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4z" /><path className="fa-primary" d="M323.8 202.5c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4s12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176z" /></svg>
            <span>{item.stem}.{item.ext}</span>
        </>
    )
}

function FileGroupImage({ item }: { item: SrcFile }) {
    return (
        <>
            <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path opacity=".4" d="M0 96C0 60.7 28.7 32 64 32l384 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm64 48a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zm2.4 258.4c4 8.3 12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4z" /><path className="fa-primary" d="M323.8 202.5c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4s12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176z" /></svg>
            <span>{item.stem}.{item.ext}</span>
        </>
    )
}

function Item({ actorRef }: { actorRef: ActorRefFrom<typeof supportedItemMachine> }) {
    const item = useSelector(actorRef, state => {
        // console.log("item state", state.context);
        return state.context
    });
    const isSelected = item.selected;
    const isExifing = useSelector(actorRef, state => state.matches("exifing"));
    // const isReady = useSelector(actorRef, state => state.matches("ready"));
    const isCommitted = useSelector(actorRef, state => state.context.isCommitted);

    return (
        <li className="grid grid-cols-[minmax(50px,1fr)_300px] items-center py-4 pl-1.5 border-b border-neutral-800">
            <div className="flex items-center whitespace-nowrap">
                {!isCommitted ? (
                    <button
                        className="py-1 px-2 mr-2 flex justify-center"
                        onClick={() => { actorRef.send({ type: item.selected ? "DESELECT_ITEM" : "SELECT_ITEM" }) }}>
                        {isSelected ? (
                            <svg className="w-4 h-4 text-amber-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM337 209L209 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L303 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>
                        ) : (
                            <svg className="w-4 h-4 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M384 64c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32L64 448c-17.7 0-32-14.3-32-32L32 96c0-17.7 14.3-32 32-32l320 0zM64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32z" /></svg>
                        )}
                    </button>
                ) : (
                    <span className="py-1 px-2 mr-2 flex justify-center">
                        <svg className="w-4 h-4 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path className="fa-secondary" opacity=".4" d="M0 96C0 60.7 28.7 32 64 32H384c35.3 0 64 28.7 64 64V416c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V96z" /><path className="fa-primary" d="" /></svg>
                    </span>
                )}
                {item.file.type === "Image" ? (
                    <FileGroupImage item={item.file.image} />
                ) : item.file.type === "Video" ? (
                    <FileGroupVideo item={item.file.video} />
                ) : item.file.type === "LiveImage" ? (
                    <FileGroupLiveImage item={item.file.image} />
                ) : (
                    <span>Error::::</span>
                )}
                <div className="ml-4 flex items-center">
                    {item.file.type === "LiveImage" && item.file.video ? (
                        <span className="flex text-neutral-500">< svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512"><path d="M0 128C0 92.7 28.7 64 64 64l256 0c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64L64 448c-35.3 0-64-28.7-64-64L0 128zM559.1 99.8c10.4 5.6 16.9 16.4 16.9 28.2l0 256c0 11.8-6.5 22.6-16.9 28.2s-23 5-32.9-1.6l-96-64L416 337.1l0-17.1 0-128 0-17.1 14.2-9.5 96-64c9.8-6.5 22.4-7.2 32.9-1.6z" /></svg></span>
                    ) : null}
                    {item.file.config.map((config: any, i: number) => (
                        <span key={i} className="mr-4 text-sm text-neutral-500">.{config.ext}</span>
                    ))}
                </div>
            </div>
            <div className="flex whitespace-nowrap">
                {!isExifing && item.file_name_next ? (
                    <div className="flex items-center">
                        <span className="text-neutral-500 mr-4">{isCommitted ? "renamed" : "rename"} to:</span>
                        <span className="mr-4">{item.file_name_next}</span>
                    </div>
                ) : null}
                <div className="w-12 flex items-center justify-center">
                    {isCommitted ? (
                        <svg className="w-5 h-5 text-green-300" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                            <path className="fa-secondary" opacity=".4" d="M0 256a256 256 0 1 0 512 0A256 256 0 1 0 0 256zm136 0c0-6.1 2.3-12.3 7-17c9.4-9.4 24.6-9.4 33.9 0l47 47c37-37 74-74 111-111c4.7-4.7 10.8-7 17-7s12.3 2.3 17 7c2.3 2.3 4.1 5 5.3 7.9c.6 1.5 1 2.9 1.3 4.4c.2 1.1 .3 2.2 .3 2.2c.1 1.2 .1 1.2 .1 2.5c-.1 1.5-.1 1.9-.1 2.3c-.1 .7-.2 1.5-.3 2.2c-.3 1.5-.7 3-1.3 4.4c-1.2 2.9-2.9 5.6-5.3 7.9c-42.7 42.7-85.3 85.3-128 128c-4.7 4.7-10.8 7-17 7s-12.3-2.3-17-7c-21.3-21.3-42.7-42.7-64-64c-4.7-4.7-7-10.8-7-17z" />
                            <path className="fa-primary" d="M369 175c9.4 9.4 9.4 24.6 0 33.9L241 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L335 175c9.4-9.4 24.6-9.4 33.9 0z" />
                        </svg>
                    ) : null}
                </div>
            </div>
        </li >
    )
}

function UnsupportedItem({ actorRef }: {
    actorRef: ActorRefFrom<typeof unsupportedItemMachine>,
}) {
    const item = useSelector(actorRef, state => state.context.file);

    return item.config.map((item) => (
        <div
            key={item.src}
            className="grid grid-cols-[minmax(50px,_300px)_minmax(100px,_1fr)] items-center py-4 pl-1.5 border-b border-neutral-800">
            <div className="flex items-center whitespace-nowrap opacity-40">
                <svg className="ml-1 w-4 mr-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 384 512"><path className="fa-secondary" opacity=".4" d="M0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-288-128 0c-17.7 0-32-14.3-32-32L224 0 64 0C28.7 0 0 28.7 0 64z" /><path className="fa-primary" d="M224 0L384 160H256c-17.7 0-32-14.3-32-32V0z" /></svg>
                <span>{item.stem}.{item.ext}</span>
            </div>
            <div className="flex whitespace-nowrap">
                <div>
                    <span className="text-neutral-500 mr-4">rename to:</span>
                    <span>- - - - - - - - - - </span>
                </div>
            </div>
        </div>
    ));
}

function UncertainItem({ actorRef }: {
    actorRef: ActorRefFrom<typeof uncertainItemMachine>
}) {
    const item = useSelector(actorRef, state => state.context.file);

    return (
        <div className="grid grid-cols-[minmax(50px,_300px)_minmax(100px,_1fr)] items-center py-4 pl-1.5 border-b border-neutral-800">
            <div>
                {item.primary.map(item => (
                    <span className="mr-3" key={item.src}>{item.stem}.{item.ext}</span>
                ))}
                {item.config.map(item => (
                    <span className="mr-3" key={item.src}>{item.stem}.{item.ext}</span>
                ))}
            </div>
        </div>
    );
}

function Rename({ actorRef }: Props) {
    console.log("we are here for some reason?");
    const source = useSelector(actorRef, (state) => {
        // console.log("rename state", state);
        return state.context.source;
    });
    const file_count = useSelector(actorRef, (state) => state.context.file_count);
    const items = useSelector(actorRef, (state) => state.context.items);
    const unsupported = useSelector(actorRef, state => state.context.unsupported);
    const uncertain = useSelector(actorRef, state => state.context.uncertain);
    const isCollecting = useSelector(actorRef, state => state.matches("collecting"));
    const isExifing = useSelector(actorRef, state => state.matches("exifing"));
    const isReady = useSelector(actorRef, state => state.matches("ready"));
    const isSelectedAll = useSelector(actorRef, state => state.context.selected_all);
    const numbOfItemsToRename = useSelector(actorRef, state => state.context.selected_count);


    return (
        <div className="">
            <div className="flex items-center justify-center mb-8">
                <p>
                    This will rename the following files to the format `YY-MM-DD_HH-MM-SS.ext`.
                </p>
            </div>
            <div className="mb-8 rounded py-2 px-4 bg-neutral-200 dark:bg-neutral-800">
                <div className="flex items-center justify-between border-b border-neutral-700 pt-2 pb-3 mb-3">
                    <div className="flex items-center">
                        <span className="mr-4"><strong className="mr-2">Source:</strong> {source}</span>
                    </div>
                    <div className="flex items-center">
                        {file_count ? (
                            <span className="text-neutral-500">has {file_count} files</span>
                        ) : null}
                    </div>
                </div>
                <div className="pb-2">
                    {isCollecting ? (
                        <div className="flex items-center">
                            <span>Collecting files</span>
                            <svg className="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                <circle className="opacity-25" stroke="currentColor" strokeWidth="4" cx="12" cy="12" r="10"></circle>
                                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                            </svg>
                        </div>
                    ) : isExifing || isReady ? (
                        <div className="flex items-center justify-between">
                            <button
                                className="flex items-center border-0 shadow-none active:bg-transparent hover:text-neutral-800 hover:dark:text-neutral-200"
                                onClick={() => actorRef.send({ type: "TOGGLE_SELECTION_ALL" })}
                            >
                                {isSelectedAll ? (
                                    <>
                                        <svg className="w-4 h-4 mr-4 text-amber-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM337 209L209 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L303 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>
                                        Deselect all
                                    </>
                                ) : (
                                    <>
                                        <svg className="w-4 h-4 mr-4 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M384 64c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32L64 448c-17.7 0-32-14.3-32-32L32 96c0-17.7 14.3-32 32-32l320 0zM64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32z" /></svg>
                                        Select all
                                    </>
                                )}
                            </button>
                            {isExifing ? (
                                <div className="flex items-center">
                                    <svg className="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                        <circle className="opacity-25" stroke="currentColor" strokeWidth="4" cx="12" cy="12" r="10"></circle>
                                        <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                    </svg>
                                    <span className="mr-2">Collecting metadata</span>
                                </div>
                            ) : isReady ? (
                                <button
                                    disabled={numbOfItemsToRename === 0}
                                    type="button"
                                    className="text-white bg-amber-700 hover:bg-amber-800 focus:ring-4 focus:outline-none focus:ring-amger-300 font-medium rounded-lg text-sm py-1.5 px-3 text-center inline-flex items-center dark:bg-amber-600 dark:hover:bg-amber-700 dark:focus:ring-amber-800 disabled:opcaity-50"
                                    onClick={() => actorRef.send({ type: "COMMIT_RENAME_GROUPS" })}
                                >
                                    Rename {numbOfItemsToRename} groups
                                </button>
                            ) : null}
                        </div>
                    ) : null}
                </div>
            </div>
            <ul>
                {items.map((item, index) => (
                    <Item actorRef={item} key={index} />
                ))}
                {uncertain.length > 0 ? (
                    <>
                        <div className="py-3 border-b border-neutral-200 dark:border-neutral-800">
                            <span className="ml-3 text-neutral-500"><span className="mr-4">---</span>Uncertain groups</span>
                        </div>
                        {uncertain.map((item, index) => (
                            <UncertainItem actorRef={item} key={`un-${index}`} />
                        ))}
                    </>
                ) : null}
                {unsupported.length > 0 ? (
                    <>
                        <div className="py-3 border-b border-neutral-200 dark:border-neutral-800">
                            <span className="ml-3 text-neutral-500"><span className="mr-4">---</span>Non media files</span>
                        </div>
                        {unsupported.map((item, index) => (
                            <UnsupportedItem actorRef={item} key={`u-${index}`} />
                        ))}
                    </>
                ) : null}
            </ul>
        </div >
    )
}

export { renameMachine };
export default Rename;
