// src/lib/sortable.js
import Sortable from "sortablejs";

export default function sortable(node, {cb, ...options}) {
    const sortableInstance = new Sortable(node, {
        ...options,
        onEnd(evt) {
            const order = Array.from(evt.from.children).map(item => item.id);
            //console.log(order);  // Logs the new order of ids
            if (cb) {
                cb(order);
            }
        }
    });

    return {
        // Cleanup sortable instance on component destroy
        destroy() {
            sortableInstance.destroy();
        }
    };
}
