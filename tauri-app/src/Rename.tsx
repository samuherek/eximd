import { ActorRefFrom, assign, fromPromise, setup } from "xstate";
import { useSelector } from "@xstate/react";
import { raiseErrorToUI } from './utils';
import { invoke } from "@tauri-apps/api";

type Props = {
    actorRef: ActorRefFrom<typeof renameMachine>
}

// const tauriActor = fromCallback(({ sendBack }) => {
//     // const unCollected = listen('FILES_COLLECTED', async (event) => {
//     //     console.log("tauri actor cllect filesl", event);
//     //     sendBack({ type: "FILES_COLLECTED", payload: event.payload });
//     // });
//     return () => {
//         // unCollected.then((fn) => fn());
//     }
// });

const tauriCollectCommand = fromPromise(async () => {
    const res = await invoke<{ files: any[], file_count: number }>("collect_rename_files");
    return {
        items: res.files,
        file_count: res.file_count
    }
});

const itemMachine = setup({
    types: {} as {
        context: {
            path: string,
            relative_path: string,
            name: string,
            ext: string,
            file_type: string,
            file_configs: any[],
            file_live_photo: any,
            selected: boolean
        },
        events: { type: "TOGGLE_ITEM" }
        | { type: "DESELECT_ITEM" }
        | { type: "SELECT_ITEM" }
    },
    guards: {
        isSelected: ({ context }) => context.selected === true,
        isDeselected: ({ context }) => context.selected === false,
    }
}).createMachine({
    context: ({ input }: { input: any }) => ({
        path: input.path,
        relative_path: input.relative_path,
        name: input.name,
        ext: input.ext,
        file_type: input.file_type,
        file_configs: input.file_configs,
        file_live_photo: input.file_live_photo,
        selected: true
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
        }
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
        | { type: "EXIF_ITEM_COLLECTED" }
    },
    actors: {
        tauriCollectCommand
    }
}).createMachine(
    {
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
                            items: ({ event, spawn }) => event.output.items.map(
                                (item) => spawn(itemMachine, {
                                    id: item.path,
                                    input: item
                                })
                            ),
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
                initial: "loading",
                states: {
                    loading: {
                        on: {
                            EXIF_ITEM_COLLECTED: {
                            }
                        }
                    },
                    done: {}
                },
            },
            ready: {
            },
            committing: {
                on: {
                    TOGGLE_SELECTION_ALL: {}
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
            }
        }
    }
);

function Item({ actorRef }: { actorRef: ActorRefFrom<typeof itemMachine> }) {
    const item = useSelector(actorRef, state => {
        console.log("item state", state);
        return state.context
    });

    return (
        <li className="flex items-center py-2 pl-4 cursor-pointer" onClick={() => actorRef.send({ type: "TOGGLE_ITEM" })}>
            {item.selected ? (
                <svg className="w-4 h-4 mr-4 text-amber-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM337 209L209 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L303 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>
            ) : (
                <svg className="w-4 h-4 mr-4 text-neutral-500" fill="currentColor" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 448 512"><path d="M384 64c17.7 0 32 14.3 32 32l0 320c0 17.7-14.3 32-32 32L64 448c-17.7 0-32-14.3-32-32L32 96c0-17.7 14.3-32 32-32l320 0zM64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32z" /></svg>
            )}
            {item.file_type === "IMG" ? (
                <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path opacity=".4" d="M0 96C0 60.7 28.7 32 64 32l384 0c35.3 0 64 28.7 64 64l0 320c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64L0 96zm64 48a48 48 0 1 0 96 0 48 48 0 1 0 -96 0zm2.4 258.4c4 8.3 12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4z" /><path className="fa-primary" d="M323.8 202.5c-4.5-6.6-11.9-10.5-19.8-10.5s-15.4 3.9-19.8 10.5l-87 127.6L170.7 297c-4.6-5.7-11.5-9-18.7-9s-14.2 3.3-18.7 9l-64 80c-5.8 7.2-6.9 17.1-2.9 25.4s12.4 13.6 21.6 13.6l96 0 32 0 208 0c8.9 0 17.1-4.9 21.2-12.8s3.6-17.4-1.4-24.7l-120-176z" /></svg>
            ) : item.file_type === "VIDEO" ? (
                <svg style={{ fill: "currentColor" }} className="w-6 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512">
                    <path className="fa-secondary" opacity=".4" d="M0 288c0 17.7 14.3 32 32 32l32 0 0 128c0 35.3 28.7 64 64 64l256 0c35.3 0 64-28.7 64-64l0-16 0-96 0-16c0-35.3-28.7-64-64-64l-32 0-224 0-64 0-32 0c-17.7 0-32 14.3-32 32z" />
                    <path className="fa-primary" d="M128 64a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM352 256l-224 0C57.3 256 0 198.7 0 128S57.3 0 128 0c48.2 0 90.2 26.6 112 66C261.8 26.6 303.8 0 352 0c70.7 0 128 57.3 128 128s-57.3 128-128 128zm0-192a64 64 0 1 0 0 128 64 64 0 1 0 0-128zM558.3 259.4c10.8 5.4 17.7 16.5 17.7 28.6l0 192c0 12.1-6.8 23.2-17.7 28.6s-23.8 4.3-33.5-3l-64-48L448 448l0-16 0-96 0-16 12.8-9.6 64-48c9.7-7.3 22.7-8.4 33.5-3z" />
                </svg>
            ) : undefined}
            <div className="flex mr-8">
                <span className="mr-4">{item.relative_path}</span>
            </div>
            <div className="flex ml-auto">
                {item.file_configs.map((config: any, i: number) => (
                    <span key={i} className="mr-4 text-sm text-neutral-500">+ {config.ext}</span>
                ))}
                {item.file_live_photo ? (
                    <span className="flex text-neutral-500">< svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 576 512"><path d="M0 128C0 92.7 28.7 64 64 64l256 0c35.3 0 64 28.7 64 64l0 256c0 35.3-28.7 64-64 64L64 448c-35.3 0-64-28.7-64-64L0 128zM559.1 99.8c10.4 5.6 16.9 16.4 16.9 28.2l0 256c0 11.8-6.5 22.6-16.9 28.2s-23 5-32.9-1.6l-96-64L416 337.1l0-17.1 0-128 0-17.1 14.2-9.5 96-64c9.8-6.5 22.4-7.2 32.9-1.6z" /></svg></span>
                ) : null}
            </div>
        </li>
    )
}

// <div className="text-neutral-500">
//     <svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M256 32a224 224 0 1 1 0 448 224 224 0 1 1 0-448zm0 480A256 256 0 1 0 256 0a256 256 0 1 0 0 512z" /></svg>
// </div>

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



    return (
        <div className="">
            <div className="flex items-center justify-center mb-12">
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
                    ) : isExifing ? (
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
                            <div className="flex items-center">
                                <svg className="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                    <circle className="opacity-25" stroke="currentColor" strokeWidth="4" cx="12" cy="12" r="10"></circle>
                                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                </svg>
                                <span className="mr-2">Collecting metadata</span>
                            </div>
                        </div>
                    ) : isReady ? (
                        <button type="button" className="text-white bg-amber-700 hover:bg-amber-800 focus:ring-4 focus:outline-none focus:ring-amger-300 font-medium rounded-lg text-sm py-1.5 px-3 text-center inline-flex items-center dark:bg-amber-600 dark:hover:bg-amber-700 dark:focus:ring-amber-800">
                            Rename xxx files
                        </button>
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
