
const BUFFER_TIME = 1.0;
// We need to retain data for some length of time for effects like delay and reverb
const CIRCULAR_BUFFER_TIME = BUFFER_TIME * 4;
const NUM_CHANNELS = 2;

/** @type {AudioContext} */
let audioContext;
/** @type {AudioWorkletNode[]} */
let processorNodes;

/** @type {{[id: number]: { data: ArrayBuffer, buf: ArrayBuffer }}} */
let channels = {};

export function createChannel(id, data){
    channels[i] = { data, buf: new Float32Array(BUFFER_SIZE) };
}

export function sendAudioJobs(jobData){
    processorNode.port.postMessage
}

export async function worker_init(numWorkers){
    audioContext = new AudioContext();
    if (audioContext.state === "running") {
        await audioContext.suspend();
    }
    await audioContext.audioWorklet.addModule("processor.js");
    
    processorNodes = Array(numWorkers);
    for(let i = 0; i < numWorkers; ++i){
        let processorNode = new AudioWorkletNode(audioContext, "cubedaw-processor", {
            processorOptions: {
                id: i,
                bufferTime: BUFFER_TIME,
                circularBufferTime: CIRCULAR_BUFFER_TIME,
            }
        });
        processorNode.connect(audioContext.destination);
        processorNode.port.onmessage = (e) => onmessage(i, e.data);
        console.log("sending init");
        processorNode.port.postMessage({ type: 'init', webassemblyData: window.workerData })
        processorNodes[i] = processorNode;
    }

    console.log("audio context", audioContext.state);
}

function onmessage(id, eventData){
    const { type, ...data } = eventData;
    switch(type) {
        case 'worker_init':
            console.log("worker init", id);
            break;
        default:
            console.warn("Invalid message type recieved:", type);
            break;
    }
}