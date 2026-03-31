//import {WSMessage} from "../../shared/types.ts";
import {WSMessage} from "./types.ts";
import {crypto} from "https://deno.land/std@0.194.0/crypto/crypto.ts";

export class SocketMap {
    public sockets: Map<string, WebSocket>;

    constructor() {
        this.sockets = new Map<string, WebSocket>();
    }

    size(): number {
        return this.sockets.size;
    }

    send(id: string, msg: WSMessage) {
        const socket = this.sockets.get(id);
        if (socket) {
            socket.send(JSON.stringify(msg));
        } else {
            console.error(`socket not found for ${id}`);
        }
    }

    gen_id(): string {
        return crypto.randomUUID();
    }

    broadcast(msg: WSMessage) {
        this.sockets.forEach((socket) => {
            if (socket.readyState === WebSocket.OPEN) {
                socket.send(JSON.stringify(msg));
            } else {
                console.error("socket not open");
            }
        });
    }

    add(id: string, socket: WebSocket) {
        console.log("adding socket", id, "to socket map");
        this.sockets.set(id, socket);
    }

    remove(id: string) {
        console.log("removing socket", id, "from socket map");
        this.sockets.delete(id);
    }
}
