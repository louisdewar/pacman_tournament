import {
    websocketConnected,
    websocketClosed,
    gameDelta,
    gameOpened,
    gameClosed,
} from './action';

export default class WebsocketManager {
    constructor(address, store) {
        this.address = address;
        this.store = store;
        this.connect.bind(this);
        this.retryDelay = 100;
        this.error = false;
        this.timeout = null;
        this.retryTimeout = null;
    }

    connect() {
        this.socket = new WebSocket(this.address);
        console.debug('Connecting websocket to', this.address);
        this.socket.onopen = this.onopen.bind(this);
        this.socket.onclose = this.onclose.bind(this);
        this.socket.onmessage = this.onmessage.bind(this);
        // Set a 3 second limit on connection initialization
        this.timeout = setTimeout(() => {
            console.debug('Socket didn\'t connect within 3 seconds');
            if (this.socket.readyState != 1 && !this.error) {
                this.store.dispatch(websocketClosed());
                this.error = true;
            }

            this.socket.close();
        }, 3000);
    }

    retry() {
        clearTimeout(this.timeout);
        if (this.socket.readyState !== 3) {
            this.socket.close();
            // This method will be called again once the socket closes
            return;
        }

        // We're already in the process of retrying
        // This probably shouldn't be necessary
        if (this.retryTimeout) {
            return;
        }

        // At this point the socket should be closed and there shouldn't be anyone
        // else trying to reconnect
        this.retryTimeout = setTimeout(() => {
            console.log('retry timeout');
            this.retryTimeout = null;
            this.connect();

            this.retryDelay *= 1.5;
            if (this.retryDelay > 2000) {
                this.retryDelay = 2000;
            }
        }, this.retryDelay);
    }

    onopen() {
        clearTimeout(this.timeout);
        console.debug('Websocket connected');
        this.store.dispatch(websocketConnected());
        this.retryDelay = 100;
        this.error = false;
    }

    onclose() {
        clearTimeout(this.timeout);
        console.log('close');
        // We haven't yet displayed an error, let's display one after a short delay
        if (!this.error) {
            setTimeout(() => {
                this.store.dispatch(websocketClosed());
                this.error = true;
            }, 500);
        }

        this.retry();
    }

    onmessage(wsMessage) {
        const msg = wsMessage.data;
        console.debug(msg);
        if (msg[0] === 'i') {
            const [gameIDStr, widthStr, heightStr, initialGameData] = msg
                .slice(1)
                .split('_');
            const [gameID, width, height] = [
                gameIDStr,
                widthStr,
                heightStr,
            ].map(s => parseInt(s));
            const [baseTiles, entitiesStr, foodStr] = initialGameData.split(
                '|'
            );

            const entities = parseSparseGrid(entitiesStr, parseMetadata);
            const food = parseSparseGrid(foodStr, parseFood);

            this.store.dispatch(
                gameOpened(gameID, width, height, baseTiles, entities, food)
            );
        } else if (msg[0] === 'd') {
            const [
                gameID,
                entity_died,
                entity_moved,
                entity_spawned,
                food_eaten,
                food_spawned,
                metadata_changed,
            ] = parseDeltas(msg.slice(1));

            this.store.dispatch(
                gameDelta(
                    gameID,
                    entity_died,
                    entity_moved,
                    entity_spawned,
                    food_eaten,
                    food_spawned,
                    metadata_changed
                )
            );
        } else if (msg[0] === 'c') {
            const gameID = parseInt(msg.slice(1));

            this.store.dispatch(gameClosed(gameID));
        } else {
            console.error('Invalid websocket message:', msg);
        }
    }
}

function parseDeltas(input) {
    // a followed by ({src},)* {letter}
    function parseDied(s) {
        if (s[0] !== 'a') {
            return [[], 0];
        }

        const died = [];

        let i = 1;
        while (i < s.length && isNumeric(s[i])) {
            const [src, n] = parseNumber(s.slice(i));
            died.push({ position: src });
            i += n;

            if (s[i] !== ',') {
                throw new Error('Missing comma after src in parse died');
            }

            i++;
        }

        return [died, i];
    }

    function parseMoved(s) {
        if (s[0] !== 'b') {
            return [[], 0];
        }

        const moved = [];

        let i = 1;

        while (i < s.length && isNumeric(s[i])) {
            const [src, srcN] = parseNumber(s.slice(i));
            i += srcN;

            if (s[i] !== ',') {
                throw new Error('Missing comma after src in parse moved');
            }

            i++;

            const [dest, destN] = parseNumber(s.slice(i));
            i += destN;

            if (s[i] !== ',') {
                throw new Error('Missing comma after dest in parse moved');
            }

            i++;

            moved.push({ src, dest });
        }

        return [moved, i];
    }

    function parseEntitySpawned(s) {
        if (s[0] !== 'c') {
            return [[], 0];
        }

        const spawned = [];

        let i = 1;

        while (i < s.length && isNumeric(s[i])) {
            const [dest, destN] = parseNumber(s.slice(i));
            i += destN;

            const [metadata, metadataN] = parseMetadata(s.slice(i));

            i += metadataN;

            spawned.push({ position: dest, metadata });
        }

        return [spawned, i];
    }

    function parseFoodEaten(s) {
        if (s[0] !== 'd') {
            return [[], 0];
        }

        const eaten = [];

        let i = 1;

        while (i < s.length && isNumeric(s[i])) {
            const [src, srcN] = parseNumber(s.slice(i));
            i += srcN;

            if (s[i] !== ',') {
                throw new Error('Missing comma after src in parse food eaten');
            }

            i++;

            eaten.push({ position: src });
        }

        return [eaten, i];
    }

    function parseFoodSpawned(s) {
        if (s[0] !== 'e') {
            return [[], 0];
        }

        const spawned = [];

        let i = 1;

        while (i < s.length && isNumeric(s[i])) {
            const [dest, destN] = parseNumber(s.slice(i));
            i += destN;
            const [type, typeN] = parseFood(s.slice(i));
            i += typeN;

            spawned.push({ position: dest, type });
        }

        return [spawned, i];
    }

    function parseMetadataChanged(s) {
        if (s[0] !== 'f') {
            return [[], 0];
        }

        const changed = [];

        let i = 1;

        while (i < s.length && isNumeric(s[i])) {
            const [dest, destN] = parseNumber(s.slice(i));
            i += destN;
            const [dynamic, dynamicN] = parseDynamicMetadata(s.slice(i));
            i += dynamicN;

            changed.push({ position: dest, dynamic });
        }

        return [changed, i];
    }

    let acc = 0;

    let [gameID, skip] = parseNumber(input[0]);
    acc += skip;

    if (input[acc] !== '_') {
        throw new Error('Missing _ after game id in parse delta');
    }

    acc++;

    let [died, skipA] = parseDied(input.slice(acc));
    acc += skipA;
    let [moved, skipB] = parseMoved(input.slice(acc));
    acc += skipB;
    let [entitySpawned, skipC] = parseEntitySpawned(input.slice(acc));
    acc += skipC;
    let [foodEaten, skipD] = parseFoodEaten(input.slice(acc));
    acc += skipD;
    let [foodSpawned, skipE] = parseFoodSpawned(input.slice(acc));
    acc += skipE;
    let [metadataChanged, skipF] = parseMetadataChanged(input.slice(acc));
    acc += skipF;

    return [
        gameID,
        died,
        moved,
        entitySpawned,
        foodEaten,
        foodSpawned,
        metadataChanged,
    ];
}

function parseFood(s) {
    return [s[0], 1];
}

// Parses the number as far as it can until it finds a non numeric char
function parseNumber(s) {
    let i = 0;
    while (isNumeric(s[i])) i++;

    return [parseInt(s.substring(0, i)), i];
}

function parseDynamicMetadata(s) {
    const direction = s[0];
    const invulnerable = s[1] === 'I';

    return [{ direction, invulnerable }, 2];
}

function parseMetadata(s) {
    const [dynamic, dynamicN] = parseDynamicMetadata(s);

    const entityType = s[dynamicN];
    const variant = parseInt(s[dynamicN + 1]);

    return [{ dynamic, entityType, variant }, 2 + dynamicN];
}

function isNumeric(c) {
    // ascii byte ranges
    if (c >= '0' && c <= '9') {
        return true;
    } else {
        return false;
    }
}

function parseSparseGrid(s, mapper) {
    let i = 0;
    const arr = [];
    while (i < s.length) {
        if (isNumeric(s[i])) {
            const [skip, n] = parseNumber(s.slice(i));
            for (let i2 = 0; i2 < skip; i2++) {
                arr.push(null);
            }
            i += n;
        } else {
            const [val, n] = mapper(s.slice(i));
            arr.push(val);
            i += n;
        }
    }

    return arr;
}
