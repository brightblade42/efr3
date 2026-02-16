
class ExpiringMap<K,V> {
    
    private map: Map<K, [V, number]>;
    private exipration: number;

    constructor(expiration: number) {
        this.map = new Map();
        this.exipration = expiration;
    }

    set(key: K, value: V) {
        //get first and skip the set if it exists. 
        //if (this.get(key)) {
        if (this.map.has(key)) {
            return;
        }
        this.map.set(key, [value, Date.now()]);
    }


    get(key: K): V | undefined {
        const val = this.map.get(key);
        if (val) {
            if (Date.now() - val[1] > this.exipration) {
                console.log(`${key} expired`)
                this.map.delete(key);
                return undefined;
            }

            console.log(`${key} cached  ${Date.now() - val[1]} remaining`); 
            return val[0];
        }
        return undefined;
    }

}

class ExpiringMapTimed<K,V> {
    
    private map: Map<K, [V, number]>;
    private expiration: number;

    constructor(expiration: number) {
        this.map = new Map();
        this.expiration = expiration;
    }

    set(key: K, value: V) {
        if (this.map.has(key)) {
            return;
        }
        this.map.set(key, [value, Date.now()]);
        setTimeout(() => {
            if (this.map.has(key)) {
                //console.log(`${key} expired`);
                this.map.delete(key);
            }
        }, this.expiration);
    }

    get(key: K): V | undefined {
        const val = this.map.get(key);
        if (val) {
           // console.log(`${key} cached  ${Date.now() - val[1]} remaining`); 
            return val[0];
        }
        return undefined;
    }
}


export { ExpiringMap, ExpiringMapTimed }