import { useSelector } from '@xstate/react';
import { useDrop } from 'react-dnd';
import { NativeTypes } from 'react-dnd-html5-backend';
import { ActorRefFrom, assign, fromCallback, fromPromise, raise, sendParent, setup } from 'xstate';
import clsx from 'clsx';
import { listen } from '@tauri-apps/api/event';
import { raiseErrorToUI } from './utils';
import { invoke } from '@tauri-apps/api';

type Props = {
    actorRef: ActorRefFrom<typeof introMachine>
}

const tauirActor = fromCallback(({ sendBack }) => {
    const unlistenDrop = listen('tauri://file-drop', async (event) => {
        sendBack({ type: "TAURI_DROP_INPUT", payload: event.payload });
    });
    return () => {
        unlistenDrop.then((fn) => fn());
    }
});

const tauriDropInputCommand = fromPromise(async ({ input }) => {
    const res = await invoke("drop_input_cmd", { payload: { items: input } });
    console.log("command res", res);
    return res;
});

const introMachine = setup({
    types: {} as {
        context: { source: string | null },
        events: { type: "TAURI_DROP_INPUT", payload: string[] }
        | { type: "CLICK_RENAME" }
        | { type: "CLICK_DEDUP" }
        | { type: "NAVIGATE" }
    },
    actors: {
        tauirActor,
        tauriDropInputCommand
    }
}).createMachine({
    id: 'intro-machine',
    context: {
        source: null,
    },
    invoke: {
        src: 'tauirActor'
    },
    type: "parallel",
    states: {
        type: {
            initial: "rename",
            states: {
                rename: {
                    on: {
                        CLICK_DEDUP: 'dedup',
                        NAVIGATE: {
                            actions: sendParent(({ context }) => ({
                                type: "NAV_RENAME",
                                payload: context.source
                            }))
                        }
                    }
                },
                dedup: {
                    on: {
                        CLICK_RENAME: 'rename',
                        NAVIGATE: {
                            actions: sendParent(({ context }) => ({
                                type: "NAV_DEDUPLICATE",
                                payload: context.source
                            }))
                        }
                    }
                }
            }
        },
        drop: {
            initial: "idle",
            states: {
                idle: {
                    on: {
                        TAURI_DROP_INPUT: {
                            target: 'loading',
                        }
                    }
                },
                loading: {
                    invoke: {
                        src: 'tauriDropInputCommand',
                        input: ({ event }: any) => event.payload,
                        onDone: {
                            actions: [assign({
                                source: ({ event }) => event.output as any,
                            }),
                            raise({ type: "NAVIGATE" })
                            ]
                        },
                        onError: {
                            actions: raiseErrorToUI
                        }
                    }
                }
            }
        }
    },
});

function Intro({ actorRef }: Props) {
    const rename = useSelector(actorRef, (state) => state.matches({ type: "rename" }));
    const dedup = useSelector(actorRef, (state) => state.matches({ type: "dedup" }));

    const [{ isOver }, dropRef] = useDrop({
        accept: NativeTypes.FILE,
        canDrop: () => true,
        collect: (monitor) => ({
            canDrop: monitor.canDrop(),
            isOver: monitor.isOver()
        })
    });

    return (
        <>
            <div className="flex items-center justify-center w-full mb-12" ref={dropRef}>
                <div className="flex flex-col items-center justify-center w-full h-64 border-2 border-neutral-700 dark:border-neutral-300 border-dashed rounded-lg"
                    style={{ borderColor: isOver ? "green" : undefined }}
                >
                    <div className="flex flex-col items-center justify-center pt-5 pb-6">
                        <svg className="w-8 h-8 mb-4 text-neutral-800 dark:text-neutral-200" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 20 16">
                            <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 13h3a3 3 0 0 0 0-6h-.025A5.56 5.56 0 0 0 16 6.5 5.5 5.5 0 0 0 5.207 5.021C5.137 5.017 5.071 5 5 5a4 4 0 0 0 0 8h2.167M10 15V6m0 0L8 8m2-2 2 2" />
                        </svg>
                        <p className="mb-2 text-sm text-neutra-800 dark:text-neutral-200"><span className="font-semibold">Drag and drop</span></p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">{rename ? "Directory or media file" : "Directory"}</p>
                    </div>
                </div>
            </div>
            <div
                className="p-8 text-md text-neutral-800 rounded-md bg-neutral-100 dark:bg-neutral-800 dark:text-neutral-300"
            >
                <h3 className="text-lg font-medium mb-4">{rename ? "File renamer" : "Duplicates finder"}</h3>
                {rename ? (
                    <p>
                        This is a simple application to rename media files. You can drag and drop a directory of all the files you would like to rename.
                        The application will present you with all the files and the new names before you commit. You can thus preview your changes first.
                    </p>
                ) : (
                    <p>
                        This is a simple application to find possible duplicates. This is meant to be used for media files and it's comaring the exif data that the camera embeded to the file when the media was taken.
                        You can see the possible previws of the files to see if they are the same.
                    </p>
                )}
            </div>
        </>
    )
}

export { introMachine };
export default Intro

