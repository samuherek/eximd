import { ActorRefFrom, assign, enqueueActions, fromCallback, fromPromise, sendParent, sendTo, setup, stopChild } from "xstate";
import { useSelector } from "@xstate/react";
import { enterFromDown, enterFromTop, LEAVE_TIME, leaveToDown, leaveToOpacity, leaveToTop, raiseErrorToUI, useNavDelay } from './utils';
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import { FileGroupToDisplay, FileGroupType, FileGroupUncertain, FileGroupUnsupported, Path, SrcFile } from "./config";
import clsx from 'clsx';

type Props = {
    actorRef: ActorRefFrom<typeof renameMachine>
}

const tauriExifCollectCommand = fromPromise<null>(async () => {
    const res = await invoke<null>("start_exif_collection_cmd");
    return res;
});

const tauriCommitRenameGroupsCommand = fromPromise<null, string[]>(async ({ input }) => {
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
        sendBack({ type: "RENAME_COMMIT_SUCCESS", payload: data.payload })
    });
    const doneUnlisten = listen<{ 
        group_count: number, 
        file_count: number 
    }>("RENAME_COMMIT_DONE_MSG", (data) => {
        sendBack({ type: "RENAME_COMMIT_DONE", payload: {
                groupCount: data.payload.group_count,
                fileCount: data.payload.file_count
        }});
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
        },
        input: FileGroupToDisplay,
        events: { type: "DESELECT_ITEM" }
        | { type: "SELECT_ITEM" }
        | { type: "SELECT_ITEM_FROM_PARENT" }
        | { type: "DESELECT_ITEM_FROM_PARENT" }
        | { type: "SET_NEXT_STEM", payload: ExifFileDataEvent }
        | { type: "RENAME_COMMIT_START" }
        | { type: "RENAME_COMMIT_SUCCESS" }
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
        RENAME_COMMIT_SUCCESS: {
            target: ".end",
        }
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
                RENAME_COMMIT_START: 'comitting',
            }
        },
        comitting: {},
        end: {
            after: {
                300: {
                    actions: [sendParent(({ context }) => ({
                        type: "REMOVE_RENMAED_ITEM",
                        payload: context.file.key
                    }))]
                }
            }
        },
    },
});

const renameMachine = setup({
    types: {} as {
        context: {
            items: ActorRefFrom<typeof supportedItemMachine>[],
            uncertain: ActorRefFrom<typeof uncertainItemMachine>[];
            unsupported: ActorRefFrom<typeof unsupportedItemMachine>[],
            source: string,
            filesCount: number,
            renameFileCount: number,
            renameGroupCount: number,
            selected_all: boolean,
            selected_count: number,
        },
        events: { type: "TOGGLE_SELECTION_ALL" }
        | { type: "EXIF_FILE_DATA", payload: ExifFileDataEvent }
        | { type: "NAV_DROP_INPUT" }
        | { type: "EXIF_COLLECTION_DONE" }
        | { type: "SELECT_ITEM" }
        | { type: "DESELECT_ITEM" }
        | { type: "SELECT_ALL" }
        | { type: "DESELECT_ALL" }
        | { type: "COMMIT_RENAME_GROUPS" }
        | { type: "RENAME_COMMIT_SUCCESS", payload: string }
        | { type: "RENAME_COMMIT_DONE", payload: { fileCount: number, groupCount: number } }
        | { type: "REMOVE_RENMAED_ITEM", payload: string }
        | { type: "NAV_RENAME" }
        | { type: "NAV_UNCERTAIN" }
        | { type: "NAV_UNSUPPORTED" }
        | { type: "NAV_ALL" },
        input: {
            fileGroups: FileGroupType[],
            source: Path,
            filesCount: number
        }
    },
    actors: {
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
        context: ({ input, spawn }) => {
            // I had to add those ts-ingore and "as any[]" as otherwise the context complained.
            // I clearly did not do the "actors" correctly although they work.
            // So I have to learn how to correctly type this.

            const [itemTypes, uncertainTypes, unsupportedTypes] = input.fileGroups.reduce((prev, next) => {
                if (next.type === "Unsupported") {
                    prev[2].push(next);
                } else if (next.type === "Uncertain") {
                    prev[1].push(next);
                } else {
                    prev[0].push(next);
                }
                return prev;
            }, [[] as FileGroupToDisplay[], [] as FileGroupUncertain[], [] as FileGroupUnsupported[]]);

            const items = itemTypes.map((item) => {
                // @ts-ignore
                return spawn(supportedItemMachine, {
                    id: item.key,
                    input: item
                })
            }) as any[];

            const uncertain = uncertainTypes.map(item => {
                // @ts-ignore
                return spawn(uncertainItemMachine, {
                    id: item.key,
                    input: item
                })
            }) as any[];

            const unsupported = unsupportedTypes.map(item => {
                // @ts-ignore
                return spawn(unsupportedItemMachine, {
                    id: item.key,
                    input: item
                })
            }) as any[];


            return {
                items: items,
                uncertain: uncertain,
                unsupported: unsupported,
                source: input.source,
                filesCount: input.filesCount,
                renameFileCount: 0,
                renameGroupCount: 0,
                selected_all: true,
                selected_count: 0,
            }
        },
        type: 'parallel',
        states: {
            view: {
                initial: 'exifing',
                states: {
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
                                on: {
                                    EXIF_FILE_DATA: {
                                        actions: sendTo(
                                            // The "key" is the key of the item machine. If it changes,
                                            // make sure to change it as well.
                                            ({ event }) => event.payload.key,
                                            ({ event }) => ({ type: "SET_NEXT_STEM", payload: event.payload })
                                        ),
                                    },
                                    EXIF_COLLECTION_DONE: '#rename-machine.view.ready',
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
                                            .map(x => x.getSnapshot().context)
                                            .filter(x => x.selected)
                                            .map(x => x.file.key);
                                        return toRename
                                    }
                                },
                            },
                            loading: {
                                entry: ({ context }) => {
                                    context.items.forEach(x => {
                                        x.send({ type: "RENAME_COMMIT_START" });
                                    });
                                },
                                on: {
                                    RENAME_COMMIT_DONE: {
                                        target: "#rename-machine.view.done",
                                        actions: [
                                            ({ event }) => console.log("TODO: set how many have been removed", event),
                                            assign({
                                                items: [],
                                                renameFileCount: ({ event }) => event.payload.fileCount,
                                                renameGroupCount: ({ event }) => event.payload.groupCount,
                                            })
                                        ]
                                    }
                                }
                            },
                            done: {}
                        },
                        on: {
                            RENAME_COMMIT_SUCCESS: {
                                actions: [
                                    sendTo(({ event }) => event.payload, { type: "RENAME_COMMIT_SUCCESS" }),
                                ]
                            },
                            REMOVE_RENMAED_ITEM: {
                                actions: [() => console.log("TODO: we are still removing"),
                                stopChild(({ event }) => event.payload),
                                assign({
                                    items: ({ context, event }) => context.items
                                        .filter(x => x.getSnapshot().context.file.key !== event.payload)
                                })
                                ]
                            },
                            TOGGLE_SELECTION_ALL: {},
                            RESET_TO_INTRO: {}
                        }
                    },
                    done: {}
                },
            },
            nav: {
                initial: 'toRename',
                states: {
                    toRename: {
                        on: {
                            NAV_RENAME: {},
                        }
                    },
                    uncertain: {
                        on: {
                            NAV_UNCERTAIN: {},
                        }
                    },
                    unsupported: {
                        on: {
                            NAV_UNSUPPORTED: {},
                        }
                    },
                    all: {
                        on: {
                            NAV_ALL: {},
                        }
                    }
                },
                on: {
                    NAV_RENAME: '.toRename',
                    NAV_UNCERTAIN: '.uncertain',
                    NAV_UNSUPPORTED: '.unsupported',
                    NAV_ALL: '.all',
                }
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
            NAV_DROP_INPUT: {
                actions: [
                    () => console.log("TODO: cleanup all threads in rust"),
                    sendParent({ type: "NAV_TO_DROP" })
                ]
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
    const isComitting = useSelector(actorRef, state => state.matches("comitting"));
    const isDone = useSelector(actorRef, state => state.matches("end"));

    return (
        <div
            className="grid grid-cols-[minmax(50px,1fr)_300px] items-center py-2 pl-1.5 border-b border-neutral-800"
            style={isDone ? leaveToOpacity() : undefined}
        >
            <div className="flex items-center whitespace-nowrap">
                <button
                    disabled={isComitting}
                    className="group w-[34px] h-[34px] ml-2 mr-2"
                    onClick={() => { actorRef.send({ type: item.selected ? "DESELECT_ITEM" : "SELECT_ITEM" }) }}>
                    <span
                        className={clsx("group-hover:scale-110 scale-100 block relative border-2 border-green-500 bg-transparent rounded-md w-[18px] h-[18px] shadow", {
                            "before:aboslute before:w-2 before:h-2 before:block before:bg-green-500 before:ml-[3px] before:mt-[3px] before:rounded-sm before:shadow": isSelected,
                            "group-hover:scale-100": isComitting,
                        })}
                    ></span>
                </button>
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
                        <ArrowRight />
                        <span className="ml-8">{item.file_name_next}</span>
                    </div>
                ) : null}
            </div>
        </div>
    )
}

function UnsupportedItem({ actorRef }: {
    actorRef: ActorRefFrom<typeof unsupportedItemMachine>,
}) {
    const item = useSelector(actorRef, state => state.context.file);

    return item.config.map((item) => (
        <div
            key={item.src}
            className="grid grid-cols-[minmax(50px,1fr)_300px]  items-center py-2 pl-1.5 border-b border-neutral-800"
        >
            <div className="flex items-center whitespace-nowrap">
                <div className="w-[34px] h-[34px] ml-2 mr-2 flex items-center">
                    <span className="block bg-neutral-300 dark:bg-neutral-700 rounded-full ml-1 w-[10px] h-[10px] shadow"></span>
                </div>
                <svg className="ml-1 w-4 mr-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 384 512"><path className="fa-secondary" opacity=".4" d="M0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-288-128 0c-17.7 0-32-14.3-32-32L224 0 64 0C28.7 0 0 28.7 0 64z" /><path className="fa-primary" d="M224 0L384 160H256c-17.7 0-32-14.3-32-32V0z" /></svg>
                <span>{item.stem}.{item.ext}</span>
            </div>
            <div className="flex whitespace-nowrap">
                <div className="flex items-center">
                    <ArrowRight />
                    <span className="ml-8">- - - - - - - - - - </span>
                </div>
            </div>
        </div>
    ));
}

function ArrowRight() {
    return (
        <svg className="h-2.5 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 11">
            <path d="M27.8422 5.88856C28.0526 5.67537 28.0526 5.32463 27.8422 5.11144L22.9539 0.159894C22.7435 -0.0532979 22.3972 -0.0532979 22.1867 0.159894C21.9763 0.373085 21.9763 0.72382 22.1867 0.937012L26.1482 4.94983H0.543136C0.244411 4.94983 0 5.1974 0 5.5C0 5.80259 0.244411 6.05017 0.543136 6.05017H26.1482L22.1867 10.063C21.9763 10.2762 21.9763 10.6269 22.1867 10.8401C22.3972 11.0533 22.7435 11.0533 22.9539 10.8401L27.8422 5.88856Z" />
        </svg>
    )
}

function UncertainItem({ actorRef }: {
    actorRef: ActorRefFrom<typeof uncertainItemMachine>
}) {
    const item = useSelector(actorRef, state => state.context.file);

    return (
        <div className="grid grid-cols-[minmax(50px,1fr)_300px]  items-center py-2 pl-1.5 border-b border-neutral-800">
            <div className="flex items-center whitespace-nowrap">
                <div className="w-[34px] h-[34px] ml-2 mr-2 flex items-center">
                    <span className="block bg-neutral-300 dark:bg-neutral-700 rounded-full ml-1 w-[10px] h-[10px] shadow"></span>
                </div>
                <svg className="ml-1 w-4 mr-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 384 512"><path className="fa-secondary" opacity=".4" d="M0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-288-128 0c-17.7 0-32-14.3-32-32L224 0 64 0C28.7 0 0 28.7 0 64z" /><path className="fa-primary" d="M224 0L384 160H256c-17.7 0-32-14.3-32-32V0z" /></svg>
                {item.primary.map(item => (
                    <span className="mr-3" key={item.src}>{item.stem}.{item.ext}</span>
                ))}
                {item.config.map(item => (
                    <span className="mr-3" key={item.src}>{item.stem}.{item.ext}</span>
                ))}
            </div>
            <div className="flex whitespace-nowrap">
                <div className="flex items-center">
                    <ArrowRight />
                </div>
            </div>
        </div>
    );
}

function Rename({ actorRef }: Props) {
    const source = useSelector(actorRef, (state) => {
        console.log("reanme state", state);
        return state.context.source;
    });
    const items = useSelector(actorRef, (state) => state.context.items);
    const unsupported = useSelector(actorRef, state => state.context.unsupported);
    const uncertain = useSelector(actorRef, state => state.context.uncertain);
    const isExifing = useSelector(actorRef, state => state.matches({ view: "exifing" }));
    const isCommitting = useSelector(actorRef, state => state.matches({ view: "committing" }));
    const isReady = useSelector(actorRef, state => state.matches({ view: "ready" }));
    const isDone = useSelector(actorRef, state => state.matches({ view: "done" }));
    const doneCount = useSelector(actorRef, state => {
        return {
            fileCount: state.context.renameFileCount,
            groupCount: state.context.renameGroupCount
        }
    });

    const [isLeaving, navDelay] = useNavDelay(LEAVE_TIME - 200);
    const nav = useSelector(actorRef, state => ({
        toRename: state.matches({ nav: "toRename" }),
        uncertain: state.matches({ nav: "uncertain" }),
        unsupported: state.matches({ nav: "unsupported" }),
        all: state.matches({ nav: "all" }),
    }))

    return (
        <>
            <div className="relative h-full">
                <div className="max-w-[1200px] mx-auto p-8">
                    <div
                        className="flex items-center justify-center pt-6 pb-8"
                        style={isLeaving ? leaveToTop({ duration: 140 }) : enterFromTop()}
                    >
                        <h2 className="relative text-2xl font-medium">
                            Rename Media Files
                            <button className="absolute ml-4 mt-1.5 text-sm text-neutral-500">
                                info
                            </button>
                        </h2>
                    </div>
                    <div
                        className={clsx("flex p-2.5 h-[56px] mb-8 rounded-lg items-center shadow-lg bg-neutral-200 dark:bg-neutral-800", {
                            "pointer-events-none": isCommitting || isDone,
                            "opacity-50": isDone
                        })}
                        style={isLeaving ? leaveToTop({ duration: 140 }) : enterFromTop()}
                    >
                        <nav>
                            <button
                                className={clsx("relative px-4 py-1.5 rounded-md font-medium text-sm mr-2", {
                                    "before:block before:absolute before:h-full before:w-1.5 before:bg-green-500 before:left-0 before:top-0 before:rounded-s-md bg-white text-black": nav.toRename,
                                    "text-neutral-300 hover:text-neutral-400": !nav.toRename
                                })}
                                onClick={() => actorRef.send({ type: "NAV_RENAME" })}
                            >
                                To Rename <span className="ml-1 text-neutral-500 text-xs">{items.length}</span>
                            </button>
                            <button
                                className={clsx("relative px-4 py-1.5 rounded-md font-medium text-sm mr-2", {
                                    "before:block before:absolute before:h-full before:w-1.5 before:bg-green-500 before:left-0 before:top-0 before:rounded-s-md bg-white text-black": nav.uncertain,
                                    "text-neutral-300 hover:text-neutral-400": !nav.uncertain
                                })}
                                onClick={() => actorRef.send({ type: "NAV_UNCERTAIN" })}
                            >
                                Uncertain <span className="ml-1 text-neutral-500 text-xs">{uncertain.length}</span>
                            </button>
                            <button
                                className={clsx("relative px-4 py-1.5 rounded-md font-medium text-sm mr-2", {
                                    "before:block before:absolute before:h-full before:w-1.5 before:bg-green-500 before:left-0 before:top-0 before:rounded-s-md bg-white text-black": nav.unsupported,
                                    "text-neutral-300 hover:text-neutral-400": !nav.unsupported
                                })}
                                onClick={() => actorRef.send({ type: "NAV_UNSUPPORTED" })}
                            >
                                Unsupported <span className="ml-1 text-neutral-500 text-xs">{unsupported.length}</span>
                            </button>
                            <button
                                className={clsx("relative px-4 py-1.5 rounded-md font-medium text-sm mr-2", {
                                    "before:block before:absolute before:h-full before:w-1.5 before:bg-green-500 before:left-0 before:top-0 before:rounded-s-md bg-white text-black": nav.all,
                                    "text-neutral-300 hover:text-neutral-400": !nav.all
                                })}
                                onClick={() => actorRef.send({ type: "NAV_ALL" })}
                            >
                                All files <span className="ml-1 text-neutral-500 text-xs">{items.length + uncertain.length + unsupported.length}</span>
                            </button>
                        </nav>
                        {isExifing || isCommitting ? (
                            <div className="ml-auto flex items-center pr-4">
                                <span className="mr-4 font-medium text-sm">{isExifing ? "Getting dates" : isCommitting ? "Renaming" : "asdsd"}</span>
                                <svg className="animate-spin text-green-500 w-5 h-5" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                                    <path opacity=".4" d="M0 256C0 114.9 114.1 .5 255.1 0C237.9 .5 224 14.6 224 32c0 17.7 14.3 32 32 32C150 64 64 150 64 256s86 192 192 192c69.7 0 130.7-37.1 164.5-92.6c-3 6.6-3.3 14.8-1 22.2c1.2 3.7 3 7.2 5.4 10.3c1.2 1.5 2.6 3 4.1 4.3c.8 .7 1.6 1.3 2.4 1.9c.4 .3 .8 .6 1.3 .9s.9 .6 1.3 .8c5 2.9 10.6 4.3 16 4.3c11 0 21.8-5.7 27.7-16c-44.3 76.5-127 128-221.7 128C114.6 512 0 397.4 0 256z" />
                                    <path d="M224 32c0-17.7 14.3-32 32-32C397.4 0 512 114.6 512 256c0 46.6-12.5 90.4-34.3 128c-8.8 15.3-28.4 20.5-43.7 11.7s-20.5-28.4-11.7-43.7c16.3-28.2 25.7-61 25.7-96c0-106-86-192-192-192c-17.7 0-32-14.3-32-32z" />
                                </svg>
                            </div>
                        ) : isReady ? (
                            <button
                                onClick={() => actorRef.send({ type: "COMMIT_RENAME_GROUPS" })}
                                disabled={isExifing}
                                className="ml-auto font-medium rounded-md px-6 py-1.5 text-black shadow-md bg-green-500 hover:bg-green-400 disabled:bg-green-300"
                            >
                                Rename
                            </button>
                        ) : null}
                    </div>

                    <div
                        className="overflow-y-auto pb-32"
                        style={{
                            maxHeight: "calc(100vh - 13rem)",
                            height: "calc(100vh - 13rem)",
                            ...(isLeaving ? leaveToDown({ duration: 140 }) : enterFromDown()),
                        }}
                    >
                        {isDone ? (
                            <div className="flex flex-col text-center pt-[10vh]"
                                style={enterFromDown()}
                            >
                                <div className="mx-auto mb-8 text-neutral-500">
                                    <svg className="w-12 h-12" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                                        <path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM129.7 327.2l28.6-14.3c8.7 17.5 41.3 55.2 97.7 55.2s88.9-37.7 97.7-55.2l28.6 14.3C369.7 352.3 327.6 400 256 400s-113.7-47.7-126.3-72.8zm89.5-89.6c-21.6-28.8-64.8-28.8-86.4 0l-25.6-19.2c34.4-45.9 103.2-45.9 137.6 0l-25.6 19.2zm73.6 0l-25.6-19.2c34.4-45.9 103.2-45.9 137.6 0l-25.6 19.2c-21.6-28.8-64.8-28.8-86.4 0z" />
                                    </svg>
                                </div>
                                <div className="mb-16">
                                    <p className="mb-6 text-lg">Succesfully renamed <span className="font-bold">{doneCount.fileCount}</span> files in <span className="font-bold">{doneCount.groupCount}</span> groups</p>
                                </div>
                                <div>
                                    <button
                                        className="rounded-md bg-green-500 px-6 py-1 font-medium text-black shadow-lg"
                                        onClick={() => navDelay(() => actorRef.send({ type: "NAV_DROP_INPUT" }))}
                                    >
                                        Start again
                                    </button>
                                </div>
                            </div>
                        ) : (
                            <>
                                {nav.toRename ? (
                                    <>
                                        {items.filter(x => x.getSnapshot().context.selected).map((item, index) => (
                                            <Item actorRef={item} key={index} />
                                        ))}
                                    </>
                                ) : nav.uncertain ? (
                                    <>
                                        {uncertain.map((item, index) => (
                                            <UncertainItem actorRef={item} key={`un-${index}`} />
                                        ))}
                                    </>
                                ) : nav.unsupported ? (
                                    <>
                                        {unsupported.map((item, index) => (
                                            <UnsupportedItem actorRef={item} key={`u-${index}`} />
                                        ))}
                                    </>
                                ) : nav.all ? (
                                    <>
                                        {items.map((item, index) => (
                                            <Item actorRef={item} key={index} />
                                        ))}
                                        {uncertain.map((item, index) => (
                                            <UncertainItem actorRef={item} key={`un-${index}`} />
                                        ))}
                                        {unsupported.map((item, index) => (
                                            <UnsupportedItem actorRef={item} key={`u-${index}`} />
                                        ))}
                                    </>
                                ) : null}
                            </>
                        )}
                    </div>
                    <div
                        className={clsx("absolute z-10 bottom-8 flex items-center p-3 px-4 rounded-lg shadow-lg bg-neutral-200 dark:bg-neutral-800", {
                            "pointer-events-none opacity-50": isCommitting || isDone
                        })}
                        style={{
                            maxHeight: "calc(100vh - 13rem)",
                            width: "calc(100vw - 4rem)",
                            maxWidth: "calc(1200px - 4rem)",
                            ...(isLeaving ? leaveToDown({ duration: 140 }) : enterFromDown())
                        }}
                    >
                        <svg className="w-4 h-4 mr-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path className="fa-secondary" opacity=".4" d="M64 480H448c35.3 0 64-28.7 64-64V160c0-35.3-28.7-64-64-64H288c-10.1 0-19.6-4.7-25.6-12.8L243.2 57.6C231.1 41.5 212.1 32 192 32H64C28.7 32 0 60.7 0 96V416c0 35.3 28.7 64 64 64z" /><path d="" /></svg>
                        <span className="">{source}</span>
                        <button
                            className="ml-auto text-sm hover:text-black hover:border-black hover:dark:text-white hover:dark:border-white dark:text-neutral-300 border px-3 py-0.5 rounded-md dark:border-neutral-400 text-neutral-700 border-neutral-600"
                            onClick={() => navDelay(() => actorRef.send({ type: "NAV_DROP_INPUT" }))}
                        >
                            Change
                        </button>
                    </div>
                </div>
            </div>
            <div className="absolute pointer-events-none bottom-0 h-36 w-full bg-gradient-to-t from-neutral-950 to-transparent"></div>
        </>
    )
}

export { renameMachine };
export default Rename;
