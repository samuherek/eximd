import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useDrop } from 'react-dnd';
import { NativeTypes } from 'react-dnd-html5-backend';
import "./App.css";
import { listen } from "@tauri-apps/api/event";
import { assign, createMachine, fromPromise } from "xstate";
import { useMachine } from "@xstate/react";
import { toast } from "react-toastify";
import { convertFileSrc } from "@tauri-apps/api/tauri";

const f = fromPromise(async ({ input }) => {
    return await invoke("drop_input", { payload: { items: input } });
});

const appMachine = createMachine({
    id: "app-machine",
    initial: "idle",
    context: {
        drop: [],
        directory: null,
        file_count: 0,
        items: [],
        errors: []
    },
    states: {
        idle: {
            on: {
                DROP_INPUT: 'loading',
            }
        },
        loading: {
            invoke: {
                src: f,
                input: ({ event }) => event.payload,
                onDone: {
                    target: 'result',
                    actions: assign({
                        items: ({ event }) => event.output.files,
                        directory: ({ event }) => event.output.directory,
                        file_count: ({ event }) => event.output.file_count,
                    })
                },
                onError: {
                    target: 'idle',
                    actions: ({ event }) => {
                        console.log("we are here, ", toast);
                        toast(event.error, {
                            type: "error"
                        })
                    }
                }
            }
        },
        result: {}
    }
});

function App() {
    const [state, send] = useMachine(appMachine);
    const [{ canDrop, isOver }, dropRef] = useDrop({
        accept: NativeTypes.FILE,
        canDrop: () => true,
        collect: (monitor) => ({
            canDrop: monitor.canDrop(),
            isOver: monitor.isOver()
        })
    });

    console.log("machine state::: ", state);

    useEffect(() => {
        const unlisten = listen('tauri://file-drop', async (event) => {
            send({ type: "DROP_INPUT", payload: event.payload });
        });
        return () => {
            unlisten.then((fn) => fn());
        }
    }, []);


    return (
        <div className="min-h-screen p-8 bg-neutral-100 dark:bg-neutral-900">
            <h1 className="text-3xl text-black dark:text-white mb-8">ExiMd</h1>
            {state.matches("idle") ? (
                <div className="flex items-center justify-center w-full"
                    ref={dropRef}
                >
                    <div className="flex flex-col items-center justify-center w-full h-64 border-2 border-neutral-700 dark:border-neutral-300 border-dashed rounded-lg"
                        style={{
                            borderColor: isOver ? "green" : undefined
                        }}
                    >
                        <div className="flex flex-col items-center justify-center pt-5 pb-6">
                            <svg className="w-8 h-8 mb-4 text-neutral-800 dark:text-neutral-200" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 20 16">
                                <path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M13 13h3a3 3 0 0 0 0-6h-.025A5.56 5.56 0 0 0 16 6.5 5.5 5.5 0 0 0 5.207 5.021C5.137 5.017 5.071 5 5 5a4 4 0 0 0 0 8h2.167M10 15V6m0 0L8 8m2-2 2 2" />
                            </svg>
                            <p className="mb-2 text-sm text-neutra-800 dark:text-neutral-200"><span className="font-semibold">Drag and drop</span></p>
                            <p className="text-xs text-gray-500 dark:text-gray-400">Directory or media file</p>
                        </div>
                    </div>
                </div>
            ) : state.matches("loading") ? (<span>loading</span>) : state.matches("result") ? (
                <div className="">
                    <div className="flex align-center justify-between mb-8 rounded py-2 px-4 bg-neutral-200 dark:bg-neutral-800">
                        <span className="mr-4"><strong className="mr-2">Source:</strong> {state.context.directory}</span>
                        <span className="text-neutral-500">({state.context.file_count} files)</span>
                    </div>
                    {state.context.items.map((item: any) => {
                        return (
                            <div key={item.path} className="flex align-center py-2 pl-4">
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
                                    <div className="text-neutral-500">
                                        <svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M256 32a224 224 0 1 1 0 448 224 224 0 1 1 0-448zm0 480A256 256 0 1 0 256 0a256 256 0 1 0 0 512z" /></svg>
                                    </div>
                                </div>
                            </div>
                        )
                    })}
                </div >
            ) : undefined
            }
        </div >
    );
}

// <svg style={{ fill: "currentColor" }} className="w-4 mr-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512"><path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM369 209L241 337c-9.4 9.4-24.6 9.4-33.9 0l-64-64c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l47 47L335 175c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9z" /></svg>

export default App;
