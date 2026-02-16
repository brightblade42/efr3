
class WebSocketClient extends EventTarget { 

    private url: string;
    private socket!: WebSocket;
    private retries = 0;
    private retryTimeout = 1000;
    private maxRetries = 10; //Infinity;
    private heartbeatInterval: number | undefined;
    private heartbeatTimeout = 30000;
    private lastMessageTimestamp = 0;
    private force_close = false;

    constructor(url: string) {
        super();
        this.url = url;
        //this.connect();
    }

    on(event: string, listener: (...args: any[]) => void) {
        this.socket.addEventListener(event, listener);
    }

    send(data: string | ArrayBufferLike | Blob | ArrayBufferView) {
        this.socket.send(data);
    }

    close() {
       this.force_close = true;
       this.retries = 0;
       this.socket.close() ;
    }
    
    private _connect() {
        this.socket = new WebSocket(this.url);

        this.socket.addEventListener('open', () => {
            console.log('WebSocket connected');
            this.dispatchEvent(new CustomEvent('connected', { detail: undefined }));
            this.retries = 0;
            //cool
            this.heartbeatInterval = window.setInterval(() => {
                if (Date.now() - this.lastMessageTimestamp > this.heartbeatTimeout) {
                    console.log('sending heartbeat msg');
                    
                    this.socket.send('ping');
                }
            }, this.heartbeatTimeout);
        });

        this.socket.addEventListener('close', (event) => {
            clearInterval(this.heartbeatInterval);
            this.dispatchEvent(new CustomEvent('disconnected', { detail: event }));

            //don't retry
            if(this.force_close) {
                console.log("socket closed permanently");
                this.force_close = false;
                return;
            }

            if (this.retries < this.maxRetries) {
                this.retries++;
                this.retryTimeout *= 1.5;
                console.log(`Retrying in ${this.retryTimeout}ms...`);
                setTimeout(() => this._connect(), this.retryTimeout);
            } else {
                console.log(`Failed to reconnect after ${this.maxRetries} attempts`);
            }
        });

        this.socket.addEventListener('error', (error) => {
            console.error('WebSocket error:', error);
        });

        this.socket.addEventListener('message', (event) => {
            this.lastMessageTimestamp = Date.now();
            //console.log('WebSocket message:', event.data);
            this.dispatchEvent(new CustomEvent('message', { detail: event.data })); 
        });
    }


    open() {
        let initRetries = 0;
        const max_retries = 3;
        let initRetryTimeout = 1000;
        let timeout_id: number | undefined;
        const tryConnect = () => {
            try {
                this._connect();
                initRetryTimeout = 1000; //we've connected, reset timer
            } catch (error) {
                console.error('Failed to connect:', error);
                if (initRetries < max_retries) {
                    initRetries++;
                    initRetryTimeout *= 1.5;
                    console.log(`Retrying initial connection in ${initRetryTimeout}ms...`);
                    timeout_id = setTimeout(tryConnect, initRetryTimeout);
                }
                else {
                    if(timeout_id) {
                        clearTimeout(timeout_id);
                        this.dispatchEvent(new CustomEvent('max_retries', { detail: undefined }));
                        console.log('Failed to establish initial connection after 3 attempts');
                    }
                }
            }
        };

        tryConnect();
    }
}

export { WebSocketClient} 