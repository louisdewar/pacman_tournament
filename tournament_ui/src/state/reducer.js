function gameReducer(newState = {}, action) {
    if (newState.error) {
        return newState;
    }

    const gameState = { ...newState['games'] } || {};
    switch (action.type) {
    case 'GAME_OPENED':
        gameState[action['gameID']] = {
            width: action['width'],
            height: action['height'],
            baseTiles: action['baseTiles'],
            entities: action['entities'],
            food: action['food'],
        };
        break;
    case 'GAME_DELTA': {
        // We may want to consider only cloning the top level game struct
        // which would involve only 3 pointer copies.
        // Then we mutate below to avoid doing loads of clones.
        // Then we ensure that components only ever render based on changes to game itself.
        const game = { ...gameState[action['gameID']] };

        const {
            entity_died,
            entity_moved,
            entity_spawned,
            food_eaten,
            food_spawned,
            metadata_changed,
        } = action;

        // Avoid wasting cpu, we don't have to copy food board if nothing changed.
        // We don't do this for entities as it is very unlikely that not entities are moving.
        if (food_eaten.length > 0 || food_spawned.length > 0) {
            game['food'] = [...game['food']];

            for (let food of food_eaten) {
                game['food'][food['position']] = null;
            }

            for (let food of food_spawned) {
                game['food'][food['position']] = food['type'];
            }
        }

        // Create a copy of entities to allow react to detect the change
        game['entities'] = [...game['entities']];

        for (let entity of entity_died) {
            game['entities'][entity['position']] = null;
        }

        for (let entity of entity_moved) {
            const src = game['entities'][entity['src']];

            if (game.entities[entity.dest] != null) {
                console.log(entity, game);
                throw new Error('dest not null');
            }

            game['entities'][entity['src']] = null;
            game['entities'][entity['dest']] = src;
        }

        for (let entity of entity_spawned) {
            game['entities'][entity['position']] = entity['metadata'];
        }

        for (let change of metadata_changed) {
            if (!game['entities'][change['position']]) {
                console.log(change);
                console.log(game['entities']);
            }
            // Dynamic is short for dynamic metadata (the metadata that changes throughout a game
            game['entities'][change['position']]['dynamic'] =
                    change['dynamic'];
        }

        gameState[action['gameID']] = game;

        break;
    }
    case 'GAME_CLOSED':
        delete gameState[action['gameID']];
        break;
    default:
        console.error('Action was not expected', action);
    }

    newState['games'] = gameState;

    return newState;
}

export default function reducer(state = {}, action) {
    const newState = { ...state };

    switch (action.type) {
    case 'WEBSOCKET_CONNECT':
        if (state.error && state.errorType != 'WEBSOCKET_CLOSED') {
            return state;
        }
        return { connection_status: 'CONNECTED' };
    case 'WEBSOCKET_CONNECTION_LOADING':
        newState['connection_status'] = 'LOADING';
        return newState;
    case 'GENERAL_ERROR':
        return { error: action['error'], errorType: action['errorType'] };
    case 'LEADERBOARD_UPDATE':
        newState['leaderboard'] = action['leaderboard'];
        return newState;
    default:
        if (action.type.startsWith('GAME_')) {
            gameReducer(newState, action);
        }
        return newState;
    }
}
