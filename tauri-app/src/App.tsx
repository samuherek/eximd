import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useDrop } from 'react-dnd';
import { NativeTypes } from 'react-dnd-html5-backend';
import "./App.css";
import { listen } from "@tauri-apps/api/event";
import { assign, createMachine, fromPromise } from "xstate";
import { useMachine } from "@xstate/react";

const f = fromPromise(async ({ input }) => {
    return await invoke("drop_input", { payload: { items: input } });
});

const appMachine = createMachine({
    id: "app-machine",
    initial: "idle",
    context: {
        drop: [],
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
                        items: ({ event }) => event.output
                    })
                },
                onError: {
                    target: 'idle',
                    actions: (...args) => console.log("errors", args)
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
    console.log("canDrop", canDrop, isOver);

    useEffect(() => {
        const unlisten = listen('tauri://file-drop', async (event) => {
            console.log("we are here", event);
            send({ type: "DROP_INPUT", payload: event.payload });
            // const res = await invoke("drop_input", { payload: { items: event.payload } });
            // console.log("response, ", res);
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
                <div>
                    {state.context.items.map(item => (
                        <div key={item}>{item}</div>
                    ))}
                </div>
            ) : undefined}
        </div >
    );
}

export default App;
