import { ActorRefFrom, assign, fromCallback, fromPromise, sendParent, sendTo, setup } from "xstate";
import { useSelector } from "@xstate/react";
import { raiseErrorToUI } from './utils';
import { invoke } from "@tauri-apps/api";
import { listen } from "@tauri-apps/api/event";
import clsx from 'clsx';

type Props = {
    actorRef: ActorRefFrom<typeof renameMachine>
}

type RelatedFile = {
    src: string,
    src_relative: string,
    stem: string,
    ext: string,
}

type FileView = {
    src: string,
    src_relative: string,
    stem: string,
    ext: string,
    file_type: string,
    file_configs: RelatedFile[],
    file_live_photo: RelatedFile,
    error: string | null
};

const tauriCollectCommand = fromPromise(async () => {
    const res = await invoke<{ files: FileView[], file_count: number }>("collect_rename_files");
    return {
        items: res.files,
        file_count: res.file_count
    }
});

const tauriExifCollectCommand = fromPromise(async () => {
    const res = await invoke("start_exif_collection");
    return res;
});

type ExifFileDataEvent = {
    index: number,
    src: string,
    src_next: string,
    stem_next: string,
}

const tauriExifDataListener = fromCallback(({ sendBack }) => {
    const unlisten = listen<{
        idx: string,
        src: string,
        src_next: string,
        stem_next: string
    }>("EXIF_FILE_DATA", (data) => {
        sendBack({
            type: "EXIF_FILE_DATA", payload: {
                index: data.payload.idx,
                src: data.payload.src,
                src_next: data.payload.src_next,
                stem_next: data.payload.stem_next
            }
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


const itemMachine = setup({
    types: {} as {
        context: FileView & {
            selected: boolean,
            src_next: string | null,
            stem_next: string | null
        },
        input: FileView,
        events: { type: "TOGGLE_ITEM" }
        | { type: "DESELECT_ITEM" }
        | { type: "SELECT_ITEM" }
        | { type: "SET_NEXT_STEM", payload: ExifFileDataEvent }
    },
    guards: {
        isSelected: ({ context }) => context.selected === true,
        isDeselected: ({ context }) => context.selected === false,
    }
}).createMachine({
    initial: "exifing",
    context: ({ input }: { input: any }) => ({
        src: input.src,
        src_relative: input.src_relative,
        stem: input.stem,
        ext: input.ext,
        file_type: input.file_type,
        file_configs: input.file_configs,
        file_live_photo: input.file_live_photo,
        selected: true,
        src_next: null,
        stem_next: null,
        error: input.error ?? null
    }),
    on: {
        TOGGLE_ITEM: {
            actions: assign({
                selected: ({ context }) => !context.selected
            })
        },
        DESELECT_ITEM: {
            guard: 'isSelected',
            actions: assign({
                selected: () => false
            })
        },
        SELECT_ITEM: {
            guard: 'isDeselected',
            actions: assign({
                selected: () => true
            })
        },
    },
    states: {
        exifing: {
            on: {
                SET_NEXT_STEM: {
                    target: 'ready',
                    actions: assign({
                        src_next: ({ event }) => event.payload.src_next,
                        stem_next: ({ event }) => event.payload.stem_next,
                    })
                }
            }
        },
        ready: {}
    }
});

const renameMachine = setup({
    types: {} as {
        context: {
            items: ActorRefFrom<typeof itemMachine>[],
            source: string,
            file_count: number | null,
            selected_all: boolean
        },
        events: { type: "TOGGLE_SELECTION_ALL" }
        | { type: "EXIF_FILE_DATA", payload: ExifFileDataEvent }
        | { type: "RESET_TO_INTRO" }
        | { type: "EXIF_COLLECTION_DONE" }
    },
    actors: {
        tauriCollectCommand,
        tauriExifCollectCommand,
        tauriExifDataListener
    }
}).createMachine(
    {
        id: "rename-machine",
        initial: 'collecting',
        context: ({ input }: any) => ({
            items: [],
            source: input.source,
            file_count: null,
            selected_all: true,
        }),
        states: {
            collecting: {
                invoke: {
                    src: 'tauriCollectCommand',
                    onDone: {
                        target: 'exifing',
                        actions: assign({
                            // @ts-ignore
                            items: ({ event, spawn }) => event.output.items.map(
                                // @ts-ignore
                                (item) => spawn(itemMachine, {
                                    id: item.src,
                                    input: item
                                })
                            )
                            ,
                            file_count: ({ event }) => event.output.file_count,
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
                                    // The "src" is the key of the item machine. If it changes,
                                    // make sure to change it as well.
                                    ({ event }) => event.payload.src,
                                    ({ event }) => ({ type: "SET_NEXT_STEM", payload: event.payload })
                                ),
                            },
                            EXIF_COLLECTION_DONE: '#rename-machine.ready'
                        }
                    },
                },
            },
            ready: {},
            committing: {
                on: {
                    TOGGLE_SELECTION_ALL: {},
                    RESET_TO_INTRO: {}
                }
            }
        },
        on: {
            TOGGLE_SELECTION_ALL: {
                actions: [
                    assign({
                        selected_all: ({ context }) => {
                            const isEverySelected = context.items.every(x => x.getSnapshot().context.selected);
                            return isEverySelected ? false : true
                        }
                    }),
                    ({ context }) => {
                        const isEverySelected = context.items.every(x => x.getSnapshot().context.selected);
                        if (isEverySelected) {
                            context.items.forEach(x => {
                                x.send({ type: "DESELECT_ITEM" });
                            })
                        } else {
                            context.items.forEach(x => {
                                x.send({ type: "SELECT_ITEM" });
                            })
                        }
                    }]
            },
            RESET_TO_INTRO: {
                actions: sendParent({ type: "RESET_TO_INTRO" })
            }
        }
    }
);

function Item({ actorRef }: { actorRef: ActorRefFrom<typeof itemMachine> }) {
    const item = useSelector(actorRef, state => {
        // console.log("item state", state.context);
        return state.context
    });
    const isDisabled = item.file_type === "OTHER";
    const isSelected = !isDisabled && item.selected;
    // const isExifing = useSelector(actorRef, state => state.matches("exifing"));
    const isReady = useSelector(actorRef, state => state.matches("ready"));

    return (
        <li className="grid grid-cols-[minmax(50px,_400px)_minmax(100px,_1fr)] items-center py-4 pl-1.5 border-b border-neutral-800">
            <div className={clsx("flex items-center whitespace-nowrap", {
                "opacity-40": isDisabled
            })}>
                <button
                    className="py-1 px-2 mr-2 flex justify-center"
                    onClick={() => !isDisabled ? actorRef.send({ type: "TOGGLE_ITEM" }) : undefined}>
                    {isSelected ? (
                        <svg className="w-4 h-4 text-amber-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM337 209L209 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L303 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>
                    ) : (
                        <svg className="w-4 h-4 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M384 64c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32L64 448c-17.7 0-32-14.3-32-32L32 96c0-17.7 14.3-32 32-32l320 0zM64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32z" /></svg>
                    )}
                </button>
                {item.file_type === "IMG" ? (
                    <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path opacity=".4" d="M0 96C0 60.7 28.7 32 64 32l384 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm64 48a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zm2.4 258.4c4 8.3 12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4z" /><path className="fa-primary" d="M323.8 202.5c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4s12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176z" /></svg>
                ) : item.file_type === "VIDEO" ? (
                    <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512">
                        <path className="fa-secondary" opacity=".4" d="M0 288c0 17.7 14.3 32 32 32l32 0 0 128c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-16 0-96 0-16c0-35.3-28.7-64-64-64l-32 0-224 0-64 0-32 0c-17.7 0-32 14.3-32 32z" />
                        <path className="fa-primary" d="M128 64a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM352 256l-224 0C57.3 256 0 198.7 0 128S57.3 0 128 0c48.2 0 90.2 26.6 112 66C261.8 26.6 303.8 0 352 0c70.7 0 128 57.3 128 128s-57.3 128-128 128zm0-192a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM558.3 259.4c10.8 5.4 17.7 16.5 17.7 28.6l0 192c0 12.1-6.8 23.2-17.7 28.6s-23.8 4.3-33.5-3l-64-48L448 448l0-16 0-96 0-16 12.8-9.6 64-48c9.7-7.3 22.7-8.4 33.5-3z" />
                    </svg>
                ) : (
                    <svg className="ml-1 w-4 mr-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 384 512"><path className="fa-secondary" opacity=".4" d="M0 64L0 448c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-288-128 0c-17.7 0-32-14.3-32-32L224 0 64 0C28.7 0 0 28.7 0 64z" /><path className="fa-primary" d="M224 0L384 160H256c-17.7 0-32-14.3-32-32V0z" /></svg>
                )}
                <span>{item.stem}.{item.ext.toLowerCase()}</span>
            </div>
            <div className="flex whitespace-nowrap">
                {isDisabled ? (
                    <div>
                        <span className="text-neutral-500 mr-4">rename to:</span>
                        <span>- - - - - - - - - - </span>
                    </div>
                ) : isReady && item.src_next ? (
                    <div className="flex items-center">
                        <span className="text-neutral-500 mr-4">rename to:</span>
                        <span className="mr-4">{item.stem_next}.{item.ext.toLowerCase()}</span>
                    </div>
                ) : null}
                <div className="ml-auto">
                    {item.file_configs.map((config: any, i: number) => (
                        <span key={i} className="mr-4 text-sm text-neutral-500">+ .{config.ext.toLowerCase()}</span>
                    ))}
                    {item.file_live_photo ? (
                        <span className="flex text-neutral-500">< svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512"><path d="M0 128C0 92.7 28.7 64 64 64l256 0c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64L64 448c-35.3 0-64-28.7-64-64L0 128zM559.1 99.8c10.4 5.6 16.9 16.4 16.9 28.2l0 256c0 11.8-6.5 22.6-16.9 28.2s-23 5-32.9-1.6l-96-64L416 337.1l0-17.1 0-128 0-17.1 14.2-9.5 96-64c9.8-6.5 22.4-7.2 32.9-1.6z" /></svg></span>
                    ) : null}
                </div>
                <div className="w-12 flex items-center justify-center">
                    {false ? (
                        <svg className="w-4 h-4" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
                            <path className="fa-secondary" opacity=".4" d="M0 256a256 256 0 1 0 512 0A256 256 0 1 0 0 256zm136 0c0-6.1 2.3-12.3 7-17c9.4-9.4 24.6-9.4 33.9 0l47 47c37-37 74-74 111-111c4.7-4.7 10.8-7 17-7s12.3 2.3 17 7c2.3 2.3 4.1 5 5.3 7.9c.6 1.5 1 2.9 1.3 4.4c.2 1.1 .3 2.2 .3 2.2c.1 1.2 .1 1.2 .1 2.5c-.1 1.5-.1 1.9-.1 2.3c-.1 .7-.2 1.5-.3 2.2c-.3 1.5-.7 3-1.3 4.4c-1.2 2.9-2.9 5.6-5.3 7.9c-42.7 42.7-85.3 85.3-128 128c-4.7 4.7-10.8 7-17 7s-12.3-2.3-17-7c-21.3-21.3-42.7-42.7-64-64c-4.7-4.7-7-10.8-7-17z" />
                            <path className="fa-primary" d="M369 175c9.4 9.4 9.4 24.6 0 33.9L241 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L335 175c9.4-9.4 24.6-9.4 33.9 0z" />
                        </svg>
                    ) : null}
                </div>
            </div>
        </li>
    )
}

function Rename({ actorRef }: Props) {
    const source = useSelector(actorRef, (state) => {
        console.log("rename state", state);
        return state.context.source;
    });
    const file_count = useSelector(actorRef, (state) => state.context.file_count);
    const items = useSelector(actorRef, (state) => state.context.items);
    const isCollecting = useSelector(actorRef, state => state.matches("collecting"));
    const isExifing = useSelector(actorRef, state => state.matches("exifing"));
    const isReady = useSelector(actorRef, state => state.matches("ready"));
    const isSelectedAll = useSelector(actorRef, state => state.context.selected_all);
    const numbOfItemsToRename = useSelector(actorRef, state => state.context.items.filter((item) => {
        let snap = item.getSnapshot();
        return snap.context.selected && snap.context.src_next !== null
    }).length);



    return (
        <div className="">
            <div className="flex items-center justify-center mb-4">
                <button
                    type="button"
                    className={"text-white items-center outline-none font-medium rounded-md text-sm py-1.5 px-3 text-center inline-flex bg-amber-700 hover:bg-amber-800 dark:bg-amber-600 dark:hover:bg-amber-700 dark:focus:ring-amber-800"}
                    onClick={() => actorRef.send({ type: "RESET_TO_INTRO" })}
                >
                    Start over
                </button>

            </div>
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
                            <span className="text-neutral-500">found {file_count} files</span>
                        ) : null}
                    </div>
                </div>
                <div className="pb-2">
                    {isCollecting ? (
                        <div className="flex items-center">
                            <span>Collecting files from the file system</span>
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
                                >
                                    Rename {numbOfItemsToRename} files
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
            </ul>
        </div >
    )
}

export { renameMachine };
export default Rename;
